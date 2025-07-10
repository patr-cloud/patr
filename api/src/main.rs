#![feature(impl_trait_in_assoc_type)]

//! The main API server for Patr.

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
#[tracing::instrument]
async fn main() {
	use api::{
		app::{self, AppState},
		db,
		redis,
		redis_publisher,
		utils::config::{self, RunningEnvironment},
	};
	use opentelemetry::trace::TracerProvider as _;
	use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig};
	use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
	use tracing::Level;
	use tracing_opentelemetry::OpenTelemetryLayer;
	use tracing_subscriber::{
		filter::LevelFilter,
		fmt::{Layer as FmtLayer, format::FmtSpan},
		prelude::*,
	};

	let config = config::parse_config();

	tracing_subscriber::registry()
		.with(
			FmtLayer::new()
				.with_span_events(FmtSpan::NONE)
				.event_format(
					tracing_subscriber::fmt::format()
						.with_ansi(true)
						.with_file(false)
						.without_time()
						.compact(),
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
			OpenTelemetryLayer::new(
				SdkTracerProvider::builder()
					.with_batch_exporter(
						SpanExporter::builder()
							.with_http()
							.with_endpoint(&config.opentelemetry.tracing.endpoint)
							.with_protocol(Protocol::Grpc)
							.build()
							.expect("Failed to install OpenTelemetry tracing pipeline"),
					)
					.with_resource(Resource::builder().with_service_name("Patr API").build())
					.build()
					.tracer("Patr API"),
			)
			.with_filter(
				tracing_subscriber::filter::Targets::new()
					.with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE)
					.with_target("frontend", LevelFilter::TRACE)
					.with_target("models", LevelFilter::TRACE),
			),
		)
		.init();

	tracing::info!("Config parsed. Running in {} mode", config.environment);

	let database = db::connect(&config.database).await;

	let redis = redis::connect(&config.redis).await;

	let state = AppState {
		database,
		redis,
		config,
	};

	db::initialize(&state)
		.await
		.expect("error initializing database");

	futures::future::join(app::serve(&state), redis_publisher::run(&state)).await;
}

#[cfg(target_arch = "wasm32")]
fn main() {
	frontend::start();
}
