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

	let database = db::connect(&DatabaseConfig {
		file,
		connection_limit: 1,
	})
	.await
	.expect("failed to open the runner database");
	db::initialize(&database)
		.await
		.expect("failed to initialize the runner database");
}
