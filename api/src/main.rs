extern crate config as config_rs;
extern crate macros as api_macros;

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

	let db_pool = db::create_connection_pool(&config).await?;
	log::debug!("Database connection pool established");

	let app = App { config, db_pool };
	db::initialize(&app).await?;
	log::debug!("Database initialized");

	task::spawn(run_scheduler(app.clone()));
	log::debug!("Schedulers initialized");

	app::start_server(app).await;

	Ok(())
}

async fn run_scheduler(app: App) {
	let mut scheduler = JobScheduler::new();

	scheduler::CONFIG.set(app).expect("CONFIG is already set");

	let jobs = scheduler::get_scheduled_jobs();

	for job in jobs {
		scheduler.add(job);
	}

	loop {
		let wait_time = scheduler.time_till_next_job();
		task::sleep(wait_time).await;
		scheduler.tick();
	}
}
