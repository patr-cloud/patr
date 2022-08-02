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

use std::sync::Arc;

use api_macros::{migrate_query, query, query_as};
use api_models::utils::Uuid;
use app::App;
use chrono::{Datelike, Utc};
use clap::{Arg, ArgMatches, Command};
use eve_rs::handlebars::Handlebars;
use futures::future;
use tokio::{fs, runtime::Builder};
use utils::{constants, logger, Error as EveError};

use crate::models::rabbitmq::{RequestMessage, WorkspaceRequestData};

type Database = sqlx::Postgres;

fn main() -> Result<(), EveError> {
	Builder::new_multi_thread()
		.enable_io()
		.enable_time()
		.thread_name(format!("{}-worker-thread", constants::APP_NAME))
		// Each CPU gets at least 2 workers to avoid idling
		.worker_threads(num_cpus::get() * 2)
		.thread_stack_size(1024 * 1024 * 10) // 10 MiB to avoid stack overflow
		.build()?
		.block_on(async_main())
}

async fn async_main() -> Result<(), EveError> {
	let args = parse_cli_args();

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

	let render_register = create_render_registry("./assets/templates/").await?;
	log::debug!("Render register initialised");

	let rabbitmq = rabbitmq::create_rabbitmq_pool(&config).await?;
	log::debug!("Rabbitmq pool initialised");

	let app = App {
		config,
		database,
		redis,
		render_register,
		rabbitmq,
	};
	service::initialize(&app);
	log::debug!("Service initialized");

	db::initialize(&app).await?;
	log::debug!("Database initialized");

	scheduler::initialize_jobs(&app);
	log::debug!("Schedulers initialized");

	if args.is_present("db-only") {
		log::info!(
			"--db-only detected. Exiting after database initialization."
		);
		return Ok(());
	}

	scheduler::domain::refresh_domain_tld_list().await?;
	log::info!("Domain TLD list initialized");

	#[cfg(feature = "sample-data")]
	if args.is_present("populate-sample-data") {
		use tokio::task;

		task::spawn(models::initialize_sample_data(app.clone()));
	}

	future::join(app::start_server(&app), rabbitmq::start_consumer(&app)).await;

	if args.is_present("init-billing-msg") {
		// Queue the payment for the current month
		log::info!(
			"Initializing billing message to trigger bills at end of month"
		);
		let now = Utc::now();
		let month = now.month();
		let year = now.year();
		let request_id = Uuid::new_v4();
		service::send_message_to_rabbit_mq(
			&RequestMessage::Workspace(
				WorkspaceRequestData::ProcessWorkspaces {
					month, /* : if month == 12 { 1 } else { month + 1 } */
					year,  /* : if month == 12 { year + 1 } else { year } */
					request_id: request_id.clone(),
				},
			),
			&app.config,
			&request_id,
		)
		.await?;
	}

	Ok(())
}

fn parse_cli_args() -> ArgMatches {
	let app = Command::new(constants::APP_NAME)
		.version(constants::APP_VERSION)
		.author(constants::APP_AUTHORS)
		.about(constants::APP_ABOUT)
		.arg(
			Arg::new("db-only")
				.long("db-only")
				.takes_value(false)
				.help("Initialises the database and quits"),
		)
		.arg(
			Arg::new("init-billing-msg")
				.long("init-billing-msg")
				.takes_value(false)
				.help(
					"Initializes the billing trigger for next month in queue",
				),
		);
	#[cfg(feature = "sample-data")]
	{
		app.arg(
			Arg::new("populate-sample-data")
				.long("populate-sample-data")
				.takes_value(false)
				.help("Initialises the database with sample data"),
		)
		.get_matches()
	}
	#[cfg(not(feature = "sample-data"))]
	{
		app.get_matches()
	}
}

async fn create_render_registry(
	template_location: &str,
) -> Result<Arc<Handlebars<'static>>, EveError> {
	let mut iterator = fs::read_dir(template_location).await?;
	let mut render_register = Handlebars::new();

	while let Some(item) = iterator.next_entry().await? {
		let path = item.path().to_string_lossy().to_string();
		render_register.register_template_file(
			path.replace(template_location, "")
				.replace(".handlebars", "")
				.replace(".hbs", "")
				.to_string()
				.as_ref(),
			path,
		)?;
	}
	Ok(Arc::new(render_register))
}
