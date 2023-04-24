use std::{
	fmt::{Debug, Formatter},
	net::SocketAddr,
	process,
	sync::Arc,
	time::{Duration, Instant},
};

use ::redis::aio::MultiplexedConnection as RedisConnection;
use axum::{
	headers,
	http::header,
	middleware::Next,
	response::Response,
	Router,
};
use deadpool_lapin::Pool as RabbitmqPool;
use http::Request;
use sqlx::Pool;
use tokio::{signal, time};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

use crate::{prelude::*, routes, utils::settings::Settings};

pub type DatabaseConnection = axum_sqlx_tx::Tx<sqlx::Postgres>;
pub type Config = Arc<Settings>;

#[derive(Clone)]
pub struct App {
	pub config: Config,
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

	let router = Router::<App>::new()
		.merge(routes::create_sub_app(app))
		.layer(axum_sqlx_tx::Layer::new(app.database.clone()))
		.layer(
			CorsLayer::new()
				.allow_methods(Any)
				.allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
				.allow_origin(AllowOrigin::mirror_request())
				.map_error(|e| Error::from(e)),
		)
		.route_layer(axum::middleware::from_fn(logger_middleware));

	log::info!(
		"Listening for connections on {}:{}",
		app.config.bind_address,
		port
	);
	axum::Server::bind(&SocketAddr::new(app.config.bind_address, port))
		.serve(
			router
				.with_state(app.clone())
				.into_make_service_with_connect_info::<SocketAddr>(),
		)
		.with_graceful_shutdown(get_shutdown_signal())
		.await;
}

async fn get_shutdown_signal() {
	signal::ctrl_c()
		.await
		.expect("Unable to install signal handler");
	println!();
	log::warn!("Recieved stop signal. Gracefully shutting down server");
	tokio::spawn(async {
		time::sleep(Duration::from_millis(2000)).await;
		log::info!("Server taking too long to quit...");
		log::info!("Press Ctrl+C again to force quit application");
		signal::ctrl_c()
			.await
			.expect("Unable to install signal handler");
		println!();
		log::warn!("Recieved stop signal again. Force shutting down server");
		log::info!("Bye");
		process::exit(-1);
	});
}

async fn logger_middleware<B>(request: Request<B>, next: Next<B>) -> Response {
	// Start measuring time to check how long a route takes to execute
	let start_time = Instant::now();

	let path = request.uri().path();
	let method = request.method();

	// Execute the next route and handle the result
	let response = next.run(request).await;

	// Log how long the request took
	let elapsed_time = start_time.elapsed();

	log_request(
		method,
		elapsed_time,
		&path,
		&response.status(),
		response
			.headers()
			.get(headers::ContentLength)
			.map(|v| v.parse().ok())
			.unwrap_or(0),
	);

	response
}

fn log_request(
	method: &http::Method,
	elapsed_time: Duration,
	path: &str,
	status: &StatusCode,
	length: usize,
) {
	log::info!(
		target: "api::requests",
		"{} {} {} {} - {}",
		method,
		path,
		status,
		if elapsed_time.as_millis() > 0 {
			format!("{} ms", elapsed_time.as_millis())
		} else {
			format!("{} μs", elapsed_time.as_micros())
		},
		length
	);
}
