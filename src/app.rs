use crate::utils::{settings::Settings, thruster_helpers::ThrusterContext};

use async_std::task;
use colored::Colorize;
use sqlx::{mysql::MySqlPool, Connection};
use std::time::Instant;
use thruster::{
	async_middleware, errors::ThrusterError, middleware_fn, App as ThrusterApp, MiddlewareNext,
	MiddlewareResult, Request, Server, ThrusterServer,
};

#[derive(Clone)]
pub struct App {
	pub config: Settings,
	pub db_pool: MySqlPool,
}

pub async fn start_server(app: App) {
	let port = app.config.port;
	let mut thruster_app = create_thruster_app(app);

	thruster_app.use_middleware("/", async_middleware!(ThrusterContext, [init_handler]));

	thruster_app.set404(async_middleware!(ThrusterContext, [unhandled_handler]));

	let server = Server::new(thruster_app);
	server.start("127.0.0.1", port);
}

pub fn create_thruster_app(app: App) -> ThrusterApp<Request, ThrusterContext, App> {
	ThrusterApp::<Request, ThrusterContext, App>::create(generate_app_context, app)
}

#[middleware_fn]
async fn init_handler(
	mut context: ThrusterContext,
	next: MiddlewareNext<ThrusterContext>,
) -> MiddlewareResult<ThrusterContext> {
	// Start measuring time to check how long a route takes to execute
	let start_time = Instant::now();

	// Get a connection from the connection pool
	let pool_connection = context.get_state().db_pool.acquire().await;
	if let Err(err) = pool_connection {
		return Err(ThrusterError {
			context,
			cause: Some(Box::new(err)),
			status: 500,
			message: String::from("unable to aquire database connection"),
		});
	}

	// Begin a transaction on that connection
	let transaction = pool_connection.unwrap().begin().await;
	if let Err(err) = transaction {
		return Err(ThrusterError {
			context,
			cause: Some(Box::new(err)),
			status: 500,
			message: String::from("unable to aquire database connection"),
		});
	}

	// Set the transaction
	context.set_db_connection(transaction.unwrap());

	// Execute the next route and handle the result
	let route_result = next(context).await;

	// Log how long the request took, then either commit or rollback the transaction
	let elapsed_time = start_time.elapsed();

	match route_result {
		Ok(mut context) => {
			let db_connection = context.take_db_connection();

			log::info!(
				"{} {} {} {} - {}",
				context.method(),
				context.url(),
				match context.get_status() {
					100..=199 => format!("{}", context.get_status()).normal(),
					200..=299 => format!("{}", context.get_status()).green(),
					300..=399 => format!("{}", context.get_status()).cyan(),
					400..=499 => format!("{}", context.get_status()).yellow(),
					500..=599 => format!("{}", context.get_status()).red(),
					_ => format!("{}", context.get_status()).purple(),
				},
				if elapsed_time.as_millis() > 0 {
					format!("{} ms", elapsed_time.as_millis())
				} else {
					format!("{} μs", elapsed_time.as_micros())
				},
				context.get_body().len()
			);

			task::spawn(async {
				if let Err(err) = db_connection.commit().await {
					log::error!("Unable to commit transaction: {}", err);
				}
			});

			Ok(context)
		}
		Err(mut err) => {
			let context = &mut err.context;
			let db_connection = context.take_db_connection();

			log::error!(target: "console", "Error while processing request: {}", err.message);
			log::info!(
				"{} {} {} {} - {}",
				context.method(),
				context.url(),
				match err.status {
					100..=199 => format!("{}", context.get_status()).normal(),
					200..=299 => format!("{}", context.get_status()).green(),
					300..=399 => format!("{}", context.get_status()).cyan(),
					400..=499 => format!("{}", context.get_status()).yellow(),
					500..=599 => format!("{}", context.get_status()).red(),
					_ => format!("{}", context.get_status()).purple(),
				},
				if elapsed_time.as_millis() > 0 {
					format!("{} ms", elapsed_time.as_millis())
				} else {
					format!("{} μs", elapsed_time.as_micros())
				},
				context.get_body().len()
			);

			task::spawn(async {
				if let Err(err) = db_connection.rollback().await {
					log::error!("Unable to rollback transaction: {}", err);
				}
			});

			Err(err)
		}
	}
}

#[middleware_fn]
async fn unhandled_handler(
	mut context: ThrusterContext,
	_next: MiddlewareNext<ThrusterContext>,
) -> MiddlewareResult<ThrusterContext> {
	context.status(404);
	context.body(&format!(
		"Cannot {} route {}",
		context.method(),
		context.url()
	));
	Ok(context)
}

fn generate_app_context(request: Request, state: &App, _: &str) -> ThrusterContext {
	ThrusterContext::new(request, state.clone())
}
