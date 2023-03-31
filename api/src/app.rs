use std::fmt::{Debug, Formatter};

use axum::Router;
use deadpool_lapin::Pool as RabbitmqPool;
use redis::aio::MultiplexedConnection as RedisConnection;
use sqlx::Pool;

use crate::{routes, utils::settings::Settings, Database};

#[derive(Clone)]
pub struct App {
	pub config: Settings,
	pub database: Pool<Database>,
	pub redis: RedisConnection,
	pub rabbitmq: RabbitmqPool,
}

impl Debug for App {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "[App]")
	}
}

pub async fn start_server(app: &App) {
	let port = app.config.port;
	let bind_address = app.config.bind_address;

	let state = app.clone();
	let app = Router::new()
		.nest("/", routes::create_sub_route(app))
		.with_state(&state);

	let server = axum::Server::bind(
		&format!("{}:{}", bind_address, port).parse().unwrap(),
	)
	.serve(app.into_make_service());

	log::info!(
		"Server listening on address: http://{}:{}",
		bind_address,
		port
	);

	if let Err(e) = server.await {
		eprintln!("server failed to start: {}", e);
	}
}
