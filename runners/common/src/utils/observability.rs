use std::collections::HashMap;

use base64::prelude::*;
use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, Protocol, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::{Resource, logs::SdkLoggerProvider};
use tracing::{Dispatch, Level, level_filters::LevelFilter};
use tracing_subscriber::{
	Layer,
	fmt::{Layer as FmtLayer, format::FmtSpan},
	layer::SubscriberExt,
};

use crate::prelude::*;

/// Sets up the OTLP log export layer for managed-mode runners.
///
/// Returns the logger provider (for flushing on shutdown) and the tracing
/// bridge layer to add to the subscriber. The caller should apply a
/// `.with_filter()` on the returned layer before adding it to the registry.
pub fn setup_tracing<E>(
	config: &RunnerSettings<E::Settings>,
) -> Result<Option<SdkLoggerProvider>, RunnerError>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	let logger_provider = if let RunnerMode::Managed {
		runner_id,
		workspace_id,
		api_token,
		..
	} = &config.mode
	{
		let loki_url = match config.environment {
			RunningEnvironment::Production => "https://loki.patr.cloud",
			RunningEnvironment::Development => "http://localhost:3003",
		};

		// Build Basic Auth header: base64(runner_id:api_token)
		let credentials = BASE64_STANDARD.encode(format!("{}:{}", runner_id, api_token.0.token()));
		let auth_header = format!("Basic {}", credentials);

		let logger_exporter = LogExporter::builder()
			.with_http()
			.with_headers(HashMap::from([("Authorization".to_string(), auth_header)]))
			.with_endpoint(format!("{}/otlp/v1/logs", loki_url))
			.with_protocol(Protocol::HttpJson)
			.build()
			.expect("Failed to build OpenTelemetry logging pipeline");

		let logger_provider = SdkLoggerProvider::builder()
			.with_batch_exporter(logger_exporter)
			.with_resource(
				Resource::builder()
					.with_service_name(format!("runner.{}", runner_id))
					.with_attributes([
						KeyValue::new("runner_id", runner_id.to_string()),
						KeyValue::new("workspace_id", workspace_id.to_string()),
						KeyValue::new("source", "runner"),
					])
					.build(),
			)
			.build();

		Some(logger_provider)
	} else {
		None
	};

	tracing::dispatcher::set_global_default(Dispatch::new(
		tracing_subscriber::registry()
			.with(
				if config.environment == RunningEnvironment::Development {
					Some(
						console_subscriber::Builder::default()
							.with_default_env()
							.server_addr((
								console_subscriber::Server::DEFAULT_IP,
								console_subscriber::Server::DEFAULT_PORT + 1,
							))
							.spawn(),
					)
				} else {
					None
				},
			)
			.with(
				FmtLayer::new()
					.with_span_events(FmtSpan::NONE)
					.event_format(
						tracing_subscriber::fmt::format()
							.with_ansi(true)
							.with_file(false)
							.without_time()
							.with_target(false)
							.with_source_location(false)
							.compact(),
					)
					.with_filter(
						tracing_subscriber::filter::Targets::new()
							.with_target(E::runner_internal_name(), LevelFilter::TRACE)
							.with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE)
							.with_target("models", LevelFilter::TRACE)
							.with_target("frontend", LevelFilter::TRACE),
					)
					.with_filter(
						// PATR_LOG_LEVEL (e.g. "info") dials down the console
						// firehose; falls back to the per-environment default.
						std::env::var("PATR_LOG_LEVEL")
							.ok()
							.and_then(|level| level.parse::<LevelFilter>().ok())
							.unwrap_or_else(|| {
								LevelFilter::from_level(
									if config.environment == RunningEnvironment::Development {
										Level::TRACE
									} else {
										Level::DEBUG
									},
								)
							}),
					),
			)
			.with(logger_provider.as_ref().map(|provider| {
				OpenTelemetryTracingBridge::new(provider)
					.with_filter(
						tracing_subscriber::filter::Targets::new()
							.with_target(E::runner_internal_name(), LevelFilter::TRACE)
							.with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE)
							.with_target("models", LevelFilter::TRACE)
							.with_target("frontend", LevelFilter::TRACE),
					)
					.with_filter(LevelFilter::from_level(
						if config.environment == RunningEnvironment::Development {
							Level::TRACE
						} else {
							Level::DEBUG
						},
					))
			})),
	))?;

	Ok(logger_provider)
}

/// Flushes and shuts down the OTLP log provider. Should be called before the
/// runner process exits to ensure buffered logs are exported.
pub fn flush_observability(logger_provider: SdkLoggerProvider) {
	_ = logger_provider.force_flush();
	_ = logger_provider.shutdown();
}
