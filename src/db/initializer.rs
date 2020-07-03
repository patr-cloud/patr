use crate::{
	app::App,
	db::{get_database_version, set_database_version},
	utils::constants,
};

use semver::Version;
use std::cmp::Ordering;

pub async fn initialize(app: &App) -> Result<(), sqlx::Error> {
	log::info!("Initializing database");

	let tables = crate::query!("SHOW TABLES;")
		.fetch_all(&app.db_pool)
		.await?;

	// If no tables exist in the database, initialize fresh
	if tables.is_empty() {
		log::warn!("No tables exist. Creating fresh");

		// Create all tables
		initialize_meta(app).await?;
		initialize_users(app).await?;

		// Set the database schema version
		set_database_version(app, &constants::DATABASE_VERSION).await?;

		log::info!("Database created fresh");
		Ok(())
	} else {
		// If it already exists, perform a migration with the known values

		let version = get_database_version(app).await?;

		match version.cmp(&constants::DATABASE_VERSION) {
			Ordering::Greater => {
				log::error!("Database version is higher than what's recognised. Exiting...");
				panic!();
			}
			Ordering::Less => {
				log::info!(
					"Migrating from {}.{}.{}",
					version.major,
					version.minor,
					version.patch
				);

				migrate_database(app, version).await?;
			}
			Ordering::Equal => {
				log::info!("Database already in the latest version. No migration required.");
			}
		}

		Ok(())
	}
}

async fn migrate_database(app: &App, db_version: Version) -> Result<(), sqlx::Error> {
	let migrations = vec!["0.0.0"];

	let mut migrating = false;

	for migration_version in migrations {
		if migration_version == db_version.to_string() {
			migrating = true;
		}
		if !migrating {
			continue;
		}
		match migration_version {
			"0.0.0" => (),
			_ => (),
		}
	}

	set_database_version(app, &constants::DATABASE_VERSION).await?;

	Ok(())
}

async fn initialize_meta(app: &App) -> Result<(), sqlx::Error> {
	crate::query!(
		r#"
		CREATE TABLE IF NOT EXISTS meta_data (
			metaId VARCHAR(100) PRIMARY KEY,
			value TEXT NOT NULL
		);
		"#
	)
	.execute(&app.db_pool)
	.await?;
	Ok(())
}

async fn initialize_users(app: &App) -> Result<(), sqlx::Error> {
	crate::query!(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			userId BINARY(16) PRIMARY KEY,
			username VARCHAR(100) UNIQUE NOT NULL,
			password BINARY(64) NOT NULL,
			email VARCHAR(320) UNIQUE NOT NULL
		);
		"#
	)
	.execute(&app.db_pool)
	.await?;
	Ok(())
}
