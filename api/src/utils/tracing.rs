use std::collections::HashMap;

use opentelemetry::{global, trace::TracerProvider as _};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{
	LogExporter,
	MetricExporter,
	Protocol,
	SpanExporter,
	WithExportConfig,
	WithHttpConfig,
	WithTonicConfig,
	tonic_types::metadata::MetadataMap,
};
use opentelemetry_sdk::{
	Resource,
	logs::SdkLoggerProvider,
	metrics::SdkMeterProvider,
	trace::SdkTracerProvider,
};
use tracing::Level;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{
	filter::LevelFilter,
	fmt::{Layer as FmtLayer, format::FmtSpan},
	prelude::*,
};

use crate::utils::config::{AppConfig, RunningEnvironment};

/// Sets up the OpenTelemetry tracing, logging, and metrics pipelines based on
/// the provided configuration. This function initializes the OpenTelemetry
/// exporters for logs, spans, and metrics, configures the tracing subscriber
/// with appropriate layers for console output and OpenTelemetry integration,
/// and sets the global tracer and meter providers for the application. The
/// function returns the logger, tracer, and meter providers, which can be used
/// to flush and shut down the pipelines when the application exits.
pub fn setup_tracing(
	config: &AppConfig,
) -> (SdkLoggerProvider, SdkTracerProvider, SdkMeterProvider) {
	let mut metadata = MetadataMap::new();
	metadata.insert("x-scope-orgid", "patr".parse().unwrap());

	let logger_exporter = LogExporter::builder()
		.with_http()
		.with_headers(HashMap::from([(
			"x-scope-orgid".to_string(),
			"patr".to_string(),
		)]))
		.with_endpoint(format!(
			"{}/otlp/v1/logs",
			config.opentelemetry.logs.endpoint
		))
		.with_protocol(Protocol::HttpJson)
		.build()
		.expect("Failed to build OpenTelemetry logging pipeline");

	let logger_provider = SdkLoggerProvider::builder()
		.with_batch_exporter(logger_exporter)
		.with_resource(Resource::builder().with_service_name("Patr API").build())
		.build();

	let span_exporter = SpanExporter::builder()
		.with_tonic()
		.with_metadata(metadata.clone())
		.with_endpoint(&config.opentelemetry.tracing.endpoint)
		.with_protocol(Protocol::Grpc)
		.build()
		.expect("Failed to install OpenTelemetry tracing pipeline");

	let tracer_provider = SdkTracerProvider::builder()
		.with_batch_exporter(span_exporter)
		.with_resource(Resource::builder().with_service_name("Patr API").build())
		.build();

	let metric_exporter = MetricExporter::builder()
		.with_tonic()
		.with_metadata(metadata)
		.with_endpoint(&config.opentelemetry.metrics.endpoint)
		.with_protocol(Protocol::Grpc)
		.build()
		.expect("Failed to build OpenTelemetry metrics pipeline");

	let meter_provider = SdkMeterProvider::builder()
		.with_periodic_exporter(metric_exporter)
		.with_resource(Resource::builder().with_service_name("Patr API").build())
		.build();

	global::set_tracer_provider(tracer_provider.clone());
	global::set_meter_provider(meter_provider.clone());

	tracing_subscriber::registry()
		.with(
			if config.environment == RunningEnvironment::Development {
				Some(console_subscriber::spawn())
			} else {
				None
			},
		)
		.with(
			FmtLayer::new()
				.with_span_events(FmtSpan::NONE)
				.event_format(
					tracing_subscriber::fmt::format()
						.pretty()
						.with_ansi(true)
						.with_file(false)
						.without_time()
						.with_target(false)
						.with_source_location(false),
				)
				.with_filter(
					tracing_subscriber::filter::Targets::new()
						.with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE)
						.with_target("frontend", LevelFilter::TRACE)
						.with_target("models", LevelFilter::TRACE),
				)
				.with_filter(LevelFilter::from_level(
					if config.environment == RunningEnvironment::Development {
						Level::TRACE
					} else {
						Level::DEBUG
					},
				)),
		)
		.with(
			OpenTelemetryLayer::new(tracer_provider.tracer("Patr API")).with_filter(
				tracing_subscriber::filter::Targets::new()
					.with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE)
					.with_target("frontend", LevelFilter::TRACE)
					.with_target("models", LevelFilter::TRACE),
			),
		)
		.with(
			OpenTelemetryTracingBridge::new(&logger_provider).with_filter(
				tracing_subscriber::filter::Targets::new()
					.with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE)
					.with_target("frontend", LevelFilter::TRACE)
					.with_target("models", LevelFilter::TRACE),
			),
		)
		.init();

	(logger_provider, tracer_provider, meter_provider)
}

/// Flushes the tracing, logging, and metrics pipelines. This should be called
/// before the application exits to ensure that all logs, spans, and metrics are
/// exported before the application shuts down.
pub fn flush_tracing(
	logger_provider: SdkLoggerProvider,
	tracer_provider: SdkTracerProvider,
	meter_provider: SdkMeterProvider,
) {
	_ = logger_provider.force_flush();
	_ = tracer_provider.force_flush();
	_ = meter_provider.force_flush();
	_ = logger_provider.shutdown();
	_ = tracer_provider.shutdown();
	_ = meter_provider.shutdown();
}
