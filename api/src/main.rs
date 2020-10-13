#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
extern crate argon2;
extern crate async_std;
extern crate colored;
extern crate config as config_rs;
extern crate hex;
extern crate job_scheduler;
extern crate jsonwebtoken;
extern crate log;
extern crate log4rs;
extern crate macros as api_macros;
extern crate once_cell;
extern crate rand;
extern crate regex;
extern crate s3;
extern crate semver;
extern crate serde;
extern crate serde_json;
extern crate sqlx;
extern crate strum;
extern crate strum_macros;
extern crate surf;
extern crate uuid;

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

use async_std::task;
use job_scheduler::JobScheduler;
use std::{error::Error, time::Duration};

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

	let db_pool = db::create_connection_pool(&config).await?;
	log::debug!("Database connection pool established");

	let app = App { config, db_pool };
	db::initialize(&app).await?;
	log::debug!("Database initialized");

	task::spawn(run_scheduler());
	log::debug!("Schedulers initialized");

	app::start_server(app).await;

	Ok(())
}

async fn run_scheduler() {
	let mut scheduler = JobScheduler::new();

	let jobs = scheduler::get_scheduled_jobs();

	for job in jobs {
		scheduler.add(job);
	}

	loop {
		let wait_time = scheduler.time_till_next_job().as_millis() as u64;
		task::sleep(Duration::from_millis(wait_time)).await;
		scheduler.tick();
	}
}
