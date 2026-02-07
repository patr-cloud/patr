#![feature(impl_trait_in_assoc_type)]

//! The main API server for Patr.

/// This module contains the main application logic. Most of the app requests,
/// states, and mounting of endpoints are done here
pub mod app;
/// This module contains the database connection logic, as well as all the
/// ORM entities.
pub mod db;
/// This module contains the models used by the API. These are the structs that
/// are used for encoding and decoding things that are not a part of the API
/// (eg, JWT).
pub mod models;
/// This module contains the Redis connection and all utilities to set and get
/// data in Redis.
pub mod redis;
/// This module is used to listen for changes in the database and publish them
/// to Redis. This is used for the real-time updates on stream requests.
pub mod redis_publisher;
/// This module contains the routes for the API. This is where the endpoints
/// are mounted.
pub mod routes;
/// This module contains all the utilities used by the API. This includes things
/// like the config parser, the [`tower::Layer`]s that are used to parse the
/// requests.
pub mod utils;

/// A prelude that re-exports commonly used items.
pub mod prelude {
	pub use macros::query;
	pub use models::{
		AppResponse,
		ErrorType,
		ProcessedApiRequest,
		api::{ApiEndpoint, WithId},
		rbac::{
			BillingPermission,
			ContainerRegistryRepositoryPermission,
			DatabasePermission,
			DeploymentPermission,
			DnsRecordPermission,
			DomainPermission,
			ManagedURLPermission,
			Permission,
			RunnerPermission,
			SecretPermission,
			StaticSitePermission,
		},
		utils::{
			BearerToken,
			DockerContentDigest,
			DockerDistributionApiVersion,
			ListResourceQuery,
			LoginId,
			OneOrMore,
			OptionalHeader,
			Uuid,
		},
	};
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub use crate::{
		app::{AppRequest, AppState, AuthenticatedAppRequest, UnprocessedAppRequest},
		redis,
		utils::{EitherExt, RouterExt, TimeoutExt, constants, layers::ClientType},
	};

	/// The type of the database connection. A mutable reference to this should
	/// be used as the parameter for database functions, since it accepts both a
	/// connection and a transaction.
	///
	/// Example:
	/// ```rust
	/// pub fn database_fn(connection: &mut DatabaseConnection) {
	///     // Do something with `connection` ....
	/// }
	/// ```
	pub type DatabaseConnection = <DatabaseType as sqlx::Database>::Connection;

	/// The type of the database transaction. This is used in requests to
	/// rollback or commit transactions based on how an endpoint responds. This
	/// currently has a static lifetime, implying that only transactions from a
	/// pooled connection is allowed.
	pub type DatabaseTransaction = sqlx::Transaction<'static, DatabaseType>;

	/// The type of the database. This is currently set to [`sqlx::Postgres`].
	/// A type alias is used here so that it can be referenced everywhere easily
	pub type DatabaseType = sqlx::Postgres;
}

use std::sync::OnceLock;

use tokio_util::sync::CancellationToken;

/// The global cancellation token that will be used to cancel the connections
/// when the runner is stopped. This token will be used to cancel all the
/// connections that are open in the runner.
#[doc(hidden)]
static GLOBAL_CANCEL_TOKEN: OnceLock<CancellationToken> = OnceLock::new();

#[tokio::main]
async fn main() {
	use app::AppState;
	use opentelemetry::{global, trace::TracerProvider as _};
	use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
	use opentelemetry_otlp::{LogExporter, Protocol, SpanExporter, WithExportConfig};
	use opentelemetry_sdk::{Resource, logs::SdkLoggerProvider, trace::SdkTracerProvider};
	use tracing::Level;
	use tracing_opentelemetry::OpenTelemetryLayer;
	use tracing_subscriber::{
		filter::LevelFilter,
		fmt::{Layer as FmtLayer, format::FmtSpan},
		prelude::*,
	};

	use crate::utils::config::RunningEnvironment;

	let config = utils::config::parse_config();

	let logger_exporter = LogExporter::builder()
		.with_tonic()
		.with_endpoint(&config.opentelemetry.logs.endpoint)
		.with_protocol(Protocol::Grpc)
		.build()
		.expect("Failed to build OpenTelemetry logging pipeline");

	let logger_provider = SdkLoggerProvider::builder()
		.with_batch_exporter(logger_exporter)
		.with_resource(Resource::builder().with_service_name("Patr API").build())
		.build();

	let span_exporter = SpanExporter::builder()
		.with_tonic()
		.with_endpoint(&config.opentelemetry.tracing.endpoint)
		.with_protocol(Protocol::Grpc)
		.build()
		.expect("Failed to install OpenTelemetry tracing pipeline");

	let tracer_provider = SdkTracerProvider::builder()
		.with_batch_exporter(span_exporter)
		.with_resource(Resource::builder().with_service_name("Patr API").build())
		.build();

	global::set_tracer_provider(tracer_provider.clone());

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
						.with_ansi(true)
						.with_file(false)
						.without_time()
						.with_target(false)
						.with_source_location(false)
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

	tracing::info!("Config parsed. Running in {} mode", config.environment);

	tokio::spawn(async move {
		let ctrl_c = async {
			tokio::signal::ctrl_c()
				.await
				.expect("Failed to listen for SIGINT")
		};

		#[cfg(unix)]
		let terminate = async {
			tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
				.expect("failed to install signal handler")
				.recv()
				.await;
		};

		#[cfg(not(unix))]
		let terminate = std::future::pending::<()>();

		tokio::select! {
			_ = ctrl_c => (),
			_ = terminate => (),
		}

		tracing::info!("Received shutdown signal, cancelling all connections");
		GLOBAL_CANCEL_TOKEN
			.get_or_init(CancellationToken::new)
			.cancel();
	});

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

	_ = logger_provider.force_flush();
	_ = tracer_provider.force_flush();
	_ = logger_provider.shutdown();
	_ = tracer_provider.shutdown();
}

/// Returns a future that completes when the global cancellation token is
/// cancelled. Use this for graceful shutdown handlers.
#[tracing::instrument]
async fn exit_signal() {
	GLOBAL_CANCEL_TOKEN
		.get_or_init(CancellationToken::new)
		.cancelled()
		.await
}
