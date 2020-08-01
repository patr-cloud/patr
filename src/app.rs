use crate::{routes, utils::settings::Settings};

use colored::Colorize;
use express_rs::listen;
use express_rs::{
	App as ThrusterApp, Context, DefaultContext, DefaultMiddleware, Error, NextHandler,
};
use sqlx::mysql::MySqlPool;
use std::{future::Future, pin::Pin, time::Instant};

#[derive(Clone)]
pub struct App {
	pub config: Settings,
	pub db_pool: MySqlPool,
}

pub async fn start_server(app: App) {
	let port = app.config.port;

	let mut thruster_app = create_thruster_app();

	thruster_app.use_middleware("/", &[DefaultMiddleware::new(init_handler)]);
	thruster_app.use_sub_app("/", routes::create_sub_app());

	listen(thruster_app, ([127, 0, 0, 1], port)).await;
}

pub fn create_thruster_app() -> ThrusterApp<DefaultContext, DefaultMiddleware> {
	ThrusterApp::<DefaultContext, DefaultMiddleware>::new()
}

fn init_handler(
	mut context: DefaultContext,
	next: NextHandler<DefaultContext>,
) -> Pin<Box<dyn Future<Output = Result<DefaultContext, Error>> + Send>> {
	Box::pin(async move {
		// Start measuring time to check how long a route takes to execute
		let start_time = Instant::now();

		/* Get a connection from the connection pool
		let pool_connection = context.get_state().db_pool.acquire().await;
		if let Err(err) = pool_connection {
			return Ok(context);
		}

		// Begin a transaction on that connection
		let transaction = pool_connection.unwrap().begin().await;
		if let Err(err) = transaction {
			return Ok(context);
		}

		// Set the transaction
		context.set_db_connection(transaction.unwrap());
		*/

		// Execute the next route and handle the result
		context = next(context).await?;

		// Log how long the request took, then either commit or rollback the transaction
		let elapsed_time = start_time.elapsed();

		log::info!(
			"{} {} {} {} - {}",
			context.get_method(),
			context.get_path(),
			match context.get_response().get_status() {
				100..=199 => format!("{}", context.get_response().get_status()).normal(),
				200..=299 => format!("{}", context.get_response().get_status()).green(),
				300..=399 => format!("{}", context.get_response().get_status()).cyan(),
				400..=499 => format!("{}", context.get_response().get_status()).yellow(),
				500..=599 => format!("{}", context.get_response().get_status()).red(),
				_ => format!("{}", context.get_response().get_status()).purple(),
			},
			if elapsed_time.as_millis() > 0 {
				format!("{} ms", elapsed_time.as_millis())
			} else {
				format!("{} μs", elapsed_time.as_micros())
			},
			context.get_body().unwrap().len()
		);

		/*
		task::spawn(async {
			if let Err(err) = db_connection.commit().await {
				log::error!("Unable to commit transaction: {}", err);
			}
		});
		*/

		Ok(context)

		/*
		match route_result {
			Ok(mut context) => {
				//let db_connection = context.take_db_connection();
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
		*/
	})
}
