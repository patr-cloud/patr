use std::{
	fmt::{Debug, Formatter},
	net::SocketAddr,
};

use axum::Router;
use axum_sqlx_tx::Layer;
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

	let app = Router::new()
		.nest("/", routes::create_sub_route(&app))
		.with_state(app.clone())
		.layer(Layer::new(app.database.clone()));

	let server = axum::Server::bind(
		&format!("{}:{}", bind_address, port).parse().unwrap(),
	)
	.serve(app.into_make_service_with_connect_info::<SocketAddr>());

	log::info!(
		"Server listening on address: http://{}:{}",
		bind_address,
		port
	);

	if let Err(e) = server.await {
		eprintln!("server failed to start: {}", e);
	}
}
