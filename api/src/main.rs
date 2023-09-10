#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]
#![feature(impl_trait_in_assoc_type)]

//! The main API server for Patr.

use std::net::SocketAddr;

use app::AppState;

/// This module contains the main application logic. Most of the app requests,
/// states, and mounting of endpoints are done here
pub mod app;
/// This module contains the database connection logic, as well as all the
/// SeaORM entities.
pub mod db;
/// This module contains the models used by the API. These are the structs that
/// are used for encoding and decoding things that are not a part of the API
/// (eg, JWT).
pub mod models;
/// This module contains the Redis connection and all utilities to set and get
/// data in Redis.
pub mod redis;
/// This module contains the routes for the API. This is where the endpoints
/// are mounted.
pub mod routes;
/// This module contains all the utilities used by the API. This includes things
/// like the config parser, the [`tower::Layer`]s that are used to parse the
/// requests.
pub mod utils;

/// A prelude that re-exports commonly used items.
pub mod prelude {
	pub use models::{
		utils::{OneOrMore, Paginated, Uuid},
		ApiEndpoint,
		ErrorType,
	};
	pub use sea_orm::prelude::*;
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub use crate::{
		app::{AppRequest, AppResponse, AppState, AuthenticatedAppRequest},
		db,
		redis,
		utils::RouterExt,
	};
}

#[tokio::main]
#[tracing::instrument]
async fn main() {
	let config = utils::config::parse_config();

	let database = db::connect(&config.database).await;

	let redis = redis::connect(&config.redis).await;

	axum::Server::bind(&config.bind_address)
		.serve(
			app::setup_routes(&AppState {
				database,
				redis,
				config,
			})
			.into_make_service_with_connect_info::<SocketAddr>(),
		)
		.with_graceful_shutdown(async {
			tokio::signal::ctrl_c()
				.await
				.expect("failed to install CTRL+C signal handler");
		})
		.await
		.unwrap();
}
