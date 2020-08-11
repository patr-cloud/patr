#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate lazy_static;
extern crate colored;
extern crate config as config_rs;
extern crate log;
extern crate log4rs;
extern crate semver;
extern crate serde;
extern crate sqlx;
extern crate async_std;
extern crate serde_json;
extern crate jsonwebtoken;
extern crate argon2;
extern crate uuid;
extern crate regex;
extern crate rand;

mod app;
mod db;
mod macros;
mod routes;
mod utils;
mod models;

use app::App;
use utils::logger;

use std::error::Error;

pub type Result<TValue> = std::result::Result<TValue, Box<dyn Error>>;

#[async_std::main]
async fn main() -> Result<()> {
	let config = utils::settings::parse_config();
	println!(
		"[TRACE]: Configuration read. Running environment set to {}",
		config.environment
	);

	logger::initialize(&config).await?;
	log::debug!("Logger initialized");

	let db = db::create_connection_pool(&config).await?;
	log::debug!("Database connection pool established");

	let app = App {
		config,
		db_pool: db,
	};
	db::initialize(&app).await?;
	log::debug!("Database initialized");

	app::start_server(app).await;

	Ok(())
}
