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
mod routes;
mod scheduler;
mod service;
mod utils;

use std::{error::Error, sync::Arc};

use api_macros::{migrate_query, query, query_as};
use app::App;
use clap::{App as ClapApp, Arg, ArgMatches};
use eve_rs::handlebars::Handlebars;
use tokio::{fs, runtime::Builder};
use utils::{constants, logger};

pub type Result<TValue> = std::result::Result<TValue, Box<dyn Error>>;
pub type Database = sqlx::Postgres;
pub type DbConnection = <Database as sqlx::Database>::Connection;

fn main() -> Result<()> {
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

async fn async_main() -> Result<()> {
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

	let redis = db::create_redis_connection(&config).await?;
	log::debug!("Redis connection pool established");

	let render_register = create_render_registry("./assets/templates/").await?;
	log::debug!("Render register initialised");

	let app = App {
		config,
		database,
		redis,
		render_register,
	};
	db::initialize(&app).await?;
	log::debug!("Database initialized");

	if args.is_present("db-only") {
		log::info!(
			"--db-only detected. Exiting after database initialization."
		);
		return Ok(());
	}

	service::initialize(&app);
	log::debug!("Service initialized");

	// TODO: add error handling here
	scheduler::domain::refresh_domain_tld_list().await;
	log::info!("Domain TLD list initialized");

	scheduler::initialize_jobs(&app);
	log::debug!("Schedulers initialized");

	#[cfg(feature = "sample-data")]
	if args.is_present("populate-sample-data") {
		use tokio::task;

		task::spawn(models::initialize_sample_data(app.clone()));
	}
	app::start_server(app).await;

	Ok(())
}

fn parse_cli_args<'a>() -> ArgMatches<'a> {
	let app = ClapApp::new(constants::APP_NAME)
		.version(constants::APP_VERSION)
		.author(constants::APP_AUTHORS)
		.about(constants::APP_ABOUT)
		.arg(
			Arg::with_name("db-only")
				.long("db-only")
				.takes_value(false)
				.multiple(false)
				.help("Initialises the database and quits"),
		);
	#[cfg(feature = "sample-data")]
	{
		app.arg(
			Arg::with_name("populate-sample-data")
				.long("populate-sample-data")
				.takes_value(false)
				.multiple(false)
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
) -> Result<Arc<Handlebars<'static>>> {
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
