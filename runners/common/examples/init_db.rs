//! Runner database initializer.
//!
//! Usage: `cargo run -p common --example init_db -- <path>`
//!
//! Creates a SQLite database at the given path with the runner's current
//! schema, so `just prepare` can describe the runner's queries against it.

use std::{env, process};

use common::{db, prelude::*};

#[tokio::main]
async fn main() {
	let Some(file) = env::args().nth(1) else {
		eprintln!("Usage: cargo run -p common --example init_db -- <path>");
		process::exit(1);
	};

	// Migrations take the runner's config, but there's no runner here, so it's a
	// placeholder around the database file.
	let config = RunnerSettings {
		workspace_id: Uuid::nil(),
		runner_id: Uuid::nil(),
		api_token: "placeholder".parse().expect("a valid placeholder token"),
		user_agent: "init-db".parse().expect("a valid placeholder user agent"),
		environment: RunningEnvironment::Development,
		database: DatabaseConfig {
			file,
			connection_limit: 1,
		},
		data: (),
	};

	let database = db::connect(&config.database)
		.await
		.expect("failed to open the runner database");
	db::initialize(&database, &config)
		.await
		.expect("failed to initialize the runner database");
}
