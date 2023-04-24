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

pub mod app;
pub mod db;
pub mod macros;
pub mod migrations;
pub mod models;
pub mod rabbitmq;
pub mod redis;
pub mod routes;
pub mod scheduler;
pub mod service;
pub mod utils;

use api_macros::{migrate_query, query};
use api_models::prelude::*;
use app::App;
use chrono::{Datelike, Utc};
use futures::future;
use tokio::runtime::Builder;
use utils::logger;

use crate::{
	app::Config,
	models::rabbitmq::BillingData,
	utils::handlebar_registry,
};

pub type Database = sqlx::Postgres;

pub mod prelude {
	pub use api_macros::{query, query_as};
	pub use api_models::prelude::*;

	pub use crate::{
		app::{App, Config, DatabaseConnection as Connection},
		db::{self, DatabaseError, DatabaseResult},
		models::{self, rbac},
		redis,
		service,
		utils::{constants, handlebar_registry, middlewares::*, validator},
		Database,
	};
}

fn main() -> Result<(), Error> {
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

async fn async_main() -> Result<(), Error> {
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
		config: Config::new(config),
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

	// initialize handlebar for emails
	handlebar_registry::initialize_handlebar_registry();

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
			&BillingData::ProcessWorkspaces {
				month: if month == 12 { 1 } else { month + 1 },
				year: if month == 12 { year + 1 } else { year },
				request_id: request_id.clone(),
			},
			&app.config,
			&request_id,
		)
		.await?;
	}

	future::join(app::start_server(&app), rabbitmq::start_consumer(&app)).await;

	Ok(())
}
