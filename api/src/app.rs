use std::{
	fmt::{Debug, Formatter},
	process,
	time::{Duration, Instant},
};

use deadpool_lapin::Pool as RabbitmqPool;
use eve_rs::{
	listen,
	App as EveApp,
	AsError,
	Context,
	Error as _,
	HttpMethod,
	NextHandler,
	Response,
};
use redis::aio::MultiplexedConnection as RedisConnection;
use sqlx::Pool;
use tokio::{signal, time};

use crate::{
	pin_fn,
	routes,
	utils::{settings::Settings, Error, EveContext, EveMiddleware},
	Database,
};

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

	let mut eve_app = create_eve_app(app);

	eve_app.set_error_handler(|response, error| {
		Box::pin(async move { eve_error_handler(response, error).await })
	});
	eve_app.use_middleware("/", get_basic_middlewares());
	eve_app.use_sub_app(&app.config.base_path, routes::create_sub_app(app));

	log::info!(
		"Listening for connections on {}:{}",
		app.config.bind_address,
		port
	);
	let shutdown_signal = Some(get_shutdown_signal());
	listen(eve_app, (app.config.bind_address, port), shutdown_signal).await;
}

pub fn create_eve_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, Error> {
	EveApp::create(EveContext::new, app.clone())
}

fn get_basic_middlewares() -> [EveMiddleware; 3] {
	[
		EveMiddleware::CustomFunction(pin_fn!(init_states)),
		EveMiddleware::CustomFunction(pin_fn!(add_cors_headers)),
		EveMiddleware::JsonParser,
	]
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

async fn eve_error_handler(
	mut response: Response,
	error: Error,
) -> Result<(), Error> {
	log::error!("Error occured while processing request: {}", error);
	response.set_content_type("application/json")?;
	response.set_status(error.status_code())?;
	response.set_body_bytes(error.body_bytes()).await?;
	Ok(())
}

async fn init_states(
	mut context: EveContext,
	next: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	// Start measuring time to check how long a route takes to execute
	let start_time = Instant::now();

	// Get a connection from the connection pool and begin a transaction on that
	// connection
	let transaction = context.get_state().database.begin().await?;

	// Set the database connection
	context.set_database_connection(transaction);
	let path = context.get_path();
	let method = context.get_method().clone();

	// Execute the next route and handle the result
	let result = next(context).await;

	// Log how long the request took, then either commit or rollback the
	// transaction
	let elapsed_time = start_time.elapsed();

	match result {
		Ok(mut context) => {
			context
				.take_database_connection()
				.commit()
				.await
				.body::<&str>("Unable to commit transaction")?;
			log_request(
				&method,
				elapsed_time,
				&path,
				&context.get_status(),
				&context
					.get_response()
					.get_header("Content-Length")
					.unwrap_or_default()
					.parse::<usize>()
					.unwrap_or(0),
			);

			Ok(context)
		}
		Err(err) => {
			log_request(
				&method,
				elapsed_time,
				&path,
				&err.status_code(),
				&err.body_bytes().len(),
			);
			Err(err)
		}
	}
}

fn log_request(
	method: &HttpMethod,
	elapsed_time: Duration,
	path: &str,
	status: &u16,
	length: &usize,
) {
	log::info!(
		target: "api::requests",
		"{} {} {} {} - {}",
		method,
		path,
		format!("{}", status),
		if elapsed_time.as_millis() > 0 {
			format!("{} ms", elapsed_time.as_millis())
		} else {
			format!("{} μs", elapsed_time.as_micros())
		},
		length
	);
}

async fn add_cors_headers(
	mut context: EveContext,
	next: NextHandler<EveContext, Error>,
) -> Result<EveContext, Error> {
	context
		.header("Access-Control-Allow-Origin", "*")?
		.header("Access-Control-Allow-Methods", "*")?
		.header("Access-Control-Allow-Headers", "Content-Type,Authorization")?;

	if context.get_method() == &HttpMethod::Options {
		context.status(200)?;
		return Ok(context);
	}

	next(context).await
}
