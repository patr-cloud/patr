use std::{fs::File, path::Path};

use sqlx::{sqlite::SqlitePoolOptions, Pool};

use crate::prelude::*;

pub mod initializer;

pub async fn connect() -> Pool<DatabaseType> {
	let path = Path::new(constants::SQLITE_DATABASE_PATH);
	if !path.exists() {
		warn!("Database file does not exist. Creating a new one");
		let _ =
			File::create(constants::SQLITE_DATABASE_PATH).expect("Failed to create database file");
	}

	info!("Connecting to database");
	SqlitePoolOptions::new()
		.max_connections(1)
		.connect(&format!("sqlite://{}", constants::SQLITE_DATABASE_PATH))
		.await
		.expect("Failed to connect to database")
}
