mod app;
mod db;
mod macros;
mod models;
mod routes;
mod scheduler;
mod utils;

use api_macros::{query, query_as};
use app::App;
// use eve_rs::handlebars::Handlebars;
use tokio::fs;
use utils::{constants, logger};

use std::{error::Error, sync::Arc};

use clap::{App as ClapApp, Arg, ArgMatches};

pub type Result<TValue> = std::result::Result<TValue, Box<dyn Error>>;

#[tokio::main]
async fn main() -> Result<()> {
	let args = parse_cli_args();

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

	// let render_register = create_render_registry("./assets/templates/").await?;
	log::debug!("Render register initialised");

	let app = App {
		config,
		mysql,
		redis,
		// render_register,
	};
	db::initialize(&app).await?;
	log::debug!("Database initialized");

	if args.is_present("db-only") {
		log::info!(
			"--db-only detected. Exiting after database initialization."
		);
		return Ok(());
	}

	scheduler::initialize_jobs(&app);
	log::debug!("Schedulers initialized");

	app::start_server(app).await;

	Ok(())
}

fn parse_cli_args<'a>() -> ArgMatches<'a> {
	ClapApp::new(constants::APP_NAME)
		.version(constants::APP_VERSION)
		.author(constants::APP_AUTHORS)
		.about(constants::APP_ABOUT)
		.arg(
			Arg::with_name("db-only")
				.long("db-only")
				.takes_value(false)
				.multiple(false)
				.help("Initialises the database and quits"),
		)
		.get_matches()
}

// async fn create_render_registry(
// 	template_location: &str,
// ) -> Result<Arc<Handlebars<'static>>> {
// 	let mut iterator = fs::read_dir(template_location).await?;
// 	let mut render_register = Handlebars::new();

// 	while let Some(item) = iterator.next_entry().await? {
// 		let path = item.path().to_string_lossy().to_string();
// 		render_register.register_template_file(
// 			path.replace(template_location, "")
// 				.replace(".handlebars", "")
// 				.to_string()
// 				.as_ref(),
// 			path,
// 		)?;
// 	}
// 	Ok(Arc::new(render_register))
// }
