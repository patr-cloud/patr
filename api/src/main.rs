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
	pub use anyhow::Context;
	pub use macros::query;
	pub use models::{
		api::WithId,
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
		utils::{OneOrMore, Paginated, Uuid},
		ApiEndpoint,
		AppResponse,
		ErrorType,
	};
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub use crate::{
		app::{
			AppRequest,
			AppState,
			AuthenticatedAppRequest,
			ProcessedApiRequest,
			UnprocessedAppRequest,
		},
		redis,
		utils::{constants, RouterExt, TimeoutExt},
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

#[tokio::main]
#[tracing::instrument]
async fn main() {
	use app::AppState;
	use opentelemetry::{trace::TracerProvider as _, KeyValue};
	use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig};
	use opentelemetry_sdk::{runtime::Tokio as OtelTokioRuntime, trace::TracerProvider, Resource};
	use tracing::Level;
	use tracing_opentelemetry::OpenTelemetryLayer;
	use tracing_subscriber::{
		filter::LevelFilter,
		fmt::{format::FmtSpan, Layer as FmtLayer},
		prelude::*,
	};

	use crate::utils::config::RunningEnvironment;

	let config = utils::config::parse_config();

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
				TracerProvider::builder()
					.with_batch_exporter(
						SpanExporter::builder()
							.with_tonic()
							.with_endpoint(&config.opentelemetry.tracing.endpoint)
							.with_protocol(Protocol::Grpc)
							.build()
							.expect("Failed to install OpenTelemetry tracing pipeline"),
						OtelTokioRuntime,
					)
					.with_resource(Resource::new([KeyValue::new("service.name", "Patr API")]))
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

/// Listen for the exit signal and stop the server when the signal is received.
#[tracing::instrument]
async fn exit_signal() {
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
	tracing::info!("Shutdown signal received, shutting down server gracefully");
}
