mod app;
mod db;
mod macros;
mod models;
mod routes;
mod scheduler;
mod utils;

use api_macros::{query, query_as};
use app::App;
use utils::logger;

use std::error::Error;

pub type Result<TValue> = std::result::Result<TValue, Box<dyn Error>>;

#[tokio::main]
async fn main() -> Result<()> {
	let config = utils::settings::parse_config();
	println!(
		"[TRACE]: Configuration read. Running environment set to {}",
		config.environment
	);

	logger::initialize(&config).await?;
	log::debug!("Logger initialized");

	let mysql = db::create_mysql_connection(&config).await?;
	log::debug!("Mysql connection pool established");

	let redis = db::create_redis_connection(&config).await?;
	log::debug!("Redis connection pool established");

	let app = App {
		config,
		mysql,
		redis,
	};
	db::initialize(&app).await?;
	log::debug!("Database initialized");

	scheduler::initialize_jobs(&app);
	log::debug!("Schedulers initialized");

	app::start_server(app).await;

	Ok(())
}
