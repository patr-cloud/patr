use crate::{
	error,
	pin_fn,
	routes,
	utils::{settings::Settings, EveContext, EveMiddleware},
};

use colored::Colorize;
use eve_rs::{
	default_middlewares::compression,
	// handlebars::Handlebars,
	listen,
	App as EveApp,
	Context,
	Error,
	NextHandler,
	Response,
};
use redis::aio::MultiplexedConnection as RedisConnection;
use sqlx::mysql::MySqlPool;
use std::{
	error::Error as StdError,
	fmt::{Debug, Formatter},
	sync::Arc,
	time::Instant,
};



#[derive(Clone)]
pub struct App {
	pub config: Settings,
	pub mysql: MySqlPool,
	pub redis: RedisConnection,
	// pub render_register: Arc<Handlebars<'static>>,
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
	eve_app.use_middleware(
		"/",
		if cfg!(debug_assertions) {
			&[
				EveMiddleware::CustomFunction(pin_fn!(init_states)),
				EveMiddleware::JsonParser,
				EveMiddleware::UrlEncodedParser,
			]
		} else {
			&[
				EveMiddleware::CustomFunction(pin_fn!(init_states)),
				EveMiddleware::Compression(
					compression::DEFAULT_COMPRESSION_LEVEL,
				),
				EveMiddleware::JsonParser,
				EveMiddleware::UrlEncodedParser,
				EveMiddleware::CookieParser,
			]
		},
	);
	eve_app.use_sub_app(&app.config.base_path, routes::create_sub_app(&app));

	log::info!("Listening for connections on 127.0.0.1:{}", port);
	let shutdown_signal = Some(futures::future::pending());
	listen(eve_app, ([127, 0, 0, 1], port), None).await;
}

pub fn create_eve_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	EveApp::create(EveContext::new, app.clone())
}

fn eve_error_handler(
	mut response: Response,
	error: Box<dyn StdError>,
) -> Response {
	log::error!(
		"Error occured while processing request: {}",
		error.to_string()
	);
	response.set_content_type("application/json");
	response.set_body(&error!(SERVER_ERROR).to_string());
	response
}

async fn init_states(
	mut context: EveContext,
	next: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
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

	if let Err(err) = context.take_mysql_connection().commit().await {
		log::error!("Unable to commit transaction: {}", err);
		return Err(Error::new(
			Some(context),
			String::from("Unable to commit transaction"),
			500,
			Box::new(err),
		));
	}

	Ok(context)
}
