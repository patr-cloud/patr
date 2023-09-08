#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]
#![feature(impl_trait_in_assoc_type)]

//! The main API server for Patr.

use std::net::SocketAddr;

use app::AppState;

mod app;
mod db;
mod models;
mod redis;
mod routes;
mod utils;

/// A prelude that re-exports commonly used items.
pub mod prelude {
	pub use models::{
		utils::{OneOrMore, Paginated, Uuid},
		ApiEndpoint,
		ErrorType,
	};
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub use crate::{
		app::{AppRequest, AppResponse, AppState, AuthenticatedAppRequest},
		utils::{extractors::*, RouterExt},
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
