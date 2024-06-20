#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]
#![feature(impl_trait_in_assoc_type, try_blocks)]

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
	use std::net::SocketAddr;

	use app::AppState;
	use opentelemetry::KeyValue;
	use opentelemetry_otlp::WithExportConfig;
	use opentelemetry_sdk::Resource;
	use tokio::net::TcpListener;
	use tracing::{Dispatch, Level};
	use tracing_opentelemetry::OpenTelemetryLayer;
	use tracing_subscriber::{
		filter::LevelFilter,
		fmt::{format::FmtSpan, Layer as FmtLayer},
		prelude::*,
	};

	use crate::utils::config::RunningEnvironment;

	let config = utils::config::parse_config();

	tracing::dispatcher::set_global_default({
		let registry = tracing_subscriber::registry().with(
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
						.with_target("models", LevelFilter::TRACE),
				)
				.with_filter(LevelFilter::from_level(
					if config.environment == RunningEnvironment::Development {
						Level::TRACE
					} else {
						Level::DEBUG
					},
				)),
		);

		let setup_opentelemetry = |opentelemetry: &utils::config::OpenTelemetryConfig| {
			OpenTelemetryLayer::new(
				{
					let pipeline = opentelemetry_otlp::new_pipeline()
						.tracing()
						.with_trace_config(opentelemetry_sdk::trace::config().with_resource(
							Resource::new(vec![KeyValue::new("service.name", "Patr API")]),
						))
						.with_exporter(
							opentelemetry_otlp::new_exporter()
								.tonic()
								.with_endpoint(&opentelemetry.endpoint),
						);
					match config.environment {
						RunningEnvironment::Development => pipeline.install_simple(),
						RunningEnvironment::Production => {
							pipeline.install_batch(opentelemetry_sdk::runtime::Tokio)
						}
					}
				}
				.expect("Failed to install OpenTelemetry tracing pipeline"),
			)
			.with_filter(
				tracing_subscriber::filter::Targets::new()
					.with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE)
					.with_target("models", LevelFilter::TRACE),
			)
		};
		#[cfg(debug_assertions)]
		{
			if let Some(opentelemetry) = &config.opentelemetry {
				Dispatch::new(registry.with(setup_opentelemetry(opentelemetry)))
			} else {
				Dispatch::new(registry)
			}
		}
		#[cfg(not(debug_assertions))]
		{
			Dispatch::new(registry.with(setup_opentelemetry(&config.opentelemetry)))
		}
	})
	.expect("Failed to set global default subscriber");

	tracing::info!("Config parsed. Running in {} mode", config.environment);

	let bind_address = config.bind_address;

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

	let router = app::setup_routes(&state)
		.await
		.into_make_service_with_connect_info::<SocketAddr>();

	let tcp_listener = TcpListener::bind(bind_address).await.unwrap();

	tracing::info!(
		"Listening for connections on {}",
		tcp_listener.local_addr().unwrap()
	);

	futures::future::join(
		async {
			axum::serve(tcp_listener, router)
				.with_graceful_shutdown(async {
					tokio::signal::ctrl_c()
						.await
						.expect("failed to install ctrl-c signal handler");
				})
				.await
				.unwrap();
		},
		async {
			redis_publisher::run(&state).await;
		},
	)
	.await;
}
