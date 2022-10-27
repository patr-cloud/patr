// #![deny(
// 	clippy::all,
// 	clippy::correctness,
// 	clippy::style,
// 	clippy::complexity,
// 	clippy::perf,
// 	clippy::pedantic,
// 	clippy::nursery,
// 	clippy::cargo
// )]
// #![allow(clippy::module_name_repetitions)]

mod app;
mod db;
mod macros;
mod migrations;
mod models;
mod rabbitmq;
mod redis;
mod routes;
mod scheduler;
mod service;
mod utils;

use api_macros::{migrate_query, query, query_as};
use api_models::utils::Uuid;
use app::App;
use chrono::{Datelike, Utc};
use tokio::{join, runtime::Builder};
use utils::{logger, Error as EveError};

use crate::models::rabbitmq::WorkspaceRequestData;

type Database = sqlx::Postgres;

fn main() -> Result<(), EveError> {
	Builder::new_multi_thread()
		.enable_io()
		.enable_time()
		.thread_name("api-worker-thread")
		// Each CPU gets at least 2 workers to avoid idling
		.worker_threads(num_cpus::get() * 2)
		.thread_stack_size(1024 * 1024 * 10) // 10 MiB to avoid stack overflow
		.build()?
		.block_on(async_main())
}

async fn async_main() -> Result<(), EveError> {
	let config = utils::settings::parse_config();
	println!(
		"[TRACE]: Configuration read. Running environment set to {}",
		config.environment
	);

	logger::initialize(&config).await?;
	log::debug!("Logger initialized");

	let database = db::create_database_connection(&config).await?;
	log::debug!("Database connection pool established");

	let redis = redis::create_redis_connection(&config).await?;
	log::debug!("Redis connection pool established");

	let rabbitmq = rabbitmq::create_rabbitmq_pool(&config).await?;
	log::debug!("Rabbitmq pool initialised");

	let app = App {
		config,
		database,
		redis,
		rabbitmq,
	};
	service::initialize(&app);
	log::debug!("Service initialized");

	db::initialize(&app).await?;
	log::debug!("Database initialized");

	scheduler::initialize_jobs(&app);
	log::debug!("Schedulers initialized");

	if std::env::args().any(|value| value == "--db-only") {
		log::info!(
			"--db-only detected. Exiting after database initialization."
		);
		return Ok(());
	}

	scheduler::domain::refresh_domain_tld_list().await?;
	log::info!("Domain TLD list initialized");

	#[cfg(feature = "sample-data")]
	if std::env::args().any(|value| value == "--populate-sample-data") {
		use tokio::task;

		task::spawn(models::initialize_sample_data(app.clone()));
	}

	if std::env::args().any(|value| value == "--init-billing-msg") {
		// Queue the payment for the current month
		log::info!(
			"Initializing billing message to trigger bills at end of month"
		);
		let now = Utc::now();
		let month = now.month();
		let year = now.year();
		let request_id = Uuid::new_v4();
		service::send_message_to_billing_queue(
			&WorkspaceRequestData::ProcessWorkspaces {
				month: if month == 12 { 1 } else { month + 1 },
				year: if month == 12 { year + 1 } else { year },
				request_id: request_id.clone(),
			},
			&app.config,
			&request_id,
		)
		.await?;
	}

	join!(app::start_server(&app), rabbitmq::start_consumer(&app),);

	Ok(())
}
