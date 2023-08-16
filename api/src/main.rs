use std::net::SocketAddr;

use app::AppState;

mod app;
mod db;
mod redis;
mod routes;
mod utils;

pub mod prelude {
	pub use tracing::{debug, error, info, instrument, trace, warn};

	pub use crate::{app::AppState, utils::extractors::*};
}

#[tokio::main]
#[tracing::instrument]
async fn main() {
	let config = utils::config::parse_config();

	let database = db::connect(&config.database).await;

	let redis = redis::connect(&config.redis).await;

	let state = AppState { database, redis };

	axum::Server::bind(&config.bind_addr)
		.serve(
			app::setup_routes(&state)
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
