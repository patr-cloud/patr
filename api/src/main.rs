mod app;
mod db;
mod macros;
mod models;
mod routes;
mod scheduler;
mod utils;

use api_macros::{query, query_as};
use app::App;
use eve_rs::handlebars::Handlebars;
use tokio::fs;
use utils::logger;

use std::{error::Error, sync::Arc};

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

	let render_register = create_render_registry("./assets/templates/").await?;
	log::debug!("Render register initialised");

	let app = App {
		config,
		mysql,
		redis,
		render_register,
	};
	db::initialize(&app).await?;
	log::debug!("Database initialized");

	scheduler::initialize_jobs(&app);
	log::debug!("Schedulers initialized");

	app::start_server(app).await;

	Ok(())
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
				.to_string()
				.as_ref(),
			path,
		)?;
	}
	Ok(Arc::new(render_register))
}
