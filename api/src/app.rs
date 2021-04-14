use crate::{
	error,
	pin_fn,
	routes,
	utils::{
		settings::Settings,
		ErrorData,
		EveContext,
		EveError as Error,
		EveMiddleware,
	},
};

use colored::Colorize;
use eve_rs::{
	handlebars::Handlebars,
	listen,
	App as EveApp,
	AsError,
	Context,
	HttpMethod,
	NextHandler,
	Response,
};
use redis::aio::MultiplexedConnection as RedisConnection;
use sqlx::mysql::MySqlPool;
use std::{
	fmt::{Debug, Formatter},
	sync::Arc,
	time::Instant,
};

#[derive(Clone)]
pub struct App {
	pub config: Settings,
	pub mysql: MySqlPool,
	pub redis: RedisConnection,
	pub render_register: Arc<Handlebars<'static>>,
}

impl Debug for App {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:#?}", self)
	}
}

pub async fn start_server(app: App) {
	let port = app.config.port;

	let mut eve_app = create_eve_app(&app);

	eve_app.set_error_handler(eve_error_handler);
	eve_app.use_middleware("/", get_basic_middlewares());
	eve_app.use_sub_app(&app.config.base_path, routes::create_sub_app(&app));

	log::info!("Listening for connections on 127.0.0.1:{}", port);
	let shutdown_signal = Some(futures::future::pending());
	listen(eve_app, ([127, 0, 0, 1], port), shutdown_signal).await;
}

pub fn create_eve_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	EveApp::create(EveContext::new, app.clone())
}

#[cfg(debug_assertions)]
fn get_basic_middlewares() -> [EveMiddleware; 4] {
	[
		EveMiddleware::CustomFunction(pin_fn!(init_states)),
		EveMiddleware::CustomFunction(pin_fn!(add_cors_headers)),
		EveMiddleware::JsonParser,
		EveMiddleware::UrlEncodedParser,
	]
}

#[cfg(not(debug_assertions))]
fn get_basic_middlewares() -> [EveMiddleware; 6] {
	[
		EveMiddleware::CustomFunction(pin_fn!(init_states)),
		EveMiddleware::CustomFunction(pin_fn!(add_cors_headers)),
		EveMiddleware::Compression(compression::DEFAULT_COMPRESSION_LEVEL),
		EveMiddleware::JsonParser,
		EveMiddleware::UrlEncodedParser,
		EveMiddleware::CookieParser,
	]
}

fn eve_error_handler(mut response: Response, error: Error) -> Response {
	log::error!(
		"Error occured while processing request: {}",
		error.get_error().to_string()
	);
	response.set_content_type("application/json");
	response.set_status(error.get_status().unwrap_or(500));
	response.set_body_bytes(
		error
			.get_body_bytes()
			.unwrap_or(error!(SERVER_ERROR).to_string().as_bytes()),
	);
	response
}

async fn init_states(
	mut context: EveContext,
	next: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	// Start measuring time to check how long a route takes to execute
	let start_time = Instant::now();

	// Get a connection from the connection pool and begin a transaction on that connection
	let transaction = context.get_state().mysql.begin().await?;

	// Set the mysql transaction
	context.set_mysql_connection(transaction);

	// Execute the next route and handle the result
	context = next(context).await?;

	// Log how long the request took, then either commit or rollback the transaction
	let elapsed_time = start_time.elapsed();

	log::info!(
		"{} {} {} {} - {}",
		context.get_method(),
		context.get_path(),
		match context.get_response().get_status() {
			100..=199 =>
				format!("{}", context.get_response().get_status()).normal(),
			200..=299 =>
				format!("{}", context.get_response().get_status()).green(),
			300..=399 =>
				format!("{}", context.get_response().get_status()).cyan(),
			400..=499 =>
				format!("{}", context.get_response().get_status()).yellow(),
			500..=599 =>
				format!("{}", context.get_response().get_status()).red(),
			_ => format!("{}", context.get_response().get_status()).purple(),
		},
		if elapsed_time.as_millis() > 0 {
			format!("{} ms", elapsed_time.as_millis())
		} else {
			format!("{} μs", elapsed_time.as_micros())
		},
		context.get_response().get_body().len()
	);

	context
		.take_mysql_connection()
		.commit()
		.await
		.body("Unable to commit transaction")?;

	Ok(context)
}

async fn add_cors_headers(
	mut context: EveContext,
	next: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	context
		.header("Access-Control-Allow-Origin", "*")
		.header("Access-Control-Allow-Methods", "*")
		.header("Access-Control-Allow-Headers", "Content-Type,Authorization");

	if context.get_method() == &HttpMethod::Options {
		return Ok(context);
	}

	next(context).await
}
