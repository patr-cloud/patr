use crate::{
	app::App,
	db::{self, get_database_version, set_database_version},
	models::rbac,
	query,
	utils::constants,
};

use semver::Version;
use std::cmp::Ordering;

pub async fn initialize(app: &App) -> Result<(), sqlx::Error> {
	log::info!("Initializing database");

	let tables = query!("SHOW TABLES;").fetch_all(&app.db_pool).await?;

	// If no tables exist in the database, initialize fresh
	if tables.is_empty() {
		log::warn!("No tables exist. Creating fresh");

		let mut transaction = app.db_pool.begin().await?;

		// Create all tables
		db::initialize_meta_pre(&mut transaction).await?;
		db::initialize_users_pre(&mut transaction).await?;
		db::initialize_organisations_pre(&mut transaction).await?;
		db::initialize_rbac_pre(&mut transaction).await?;

		db::initialize_rbac_post(&mut transaction).await?;
		db::initialize_organisations_post(&mut transaction).await?;
		db::initialize_users_post(&mut transaction).await?;
		db::initialize_meta_post(&mut transaction).await?;

		transaction.commit().await?;

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

		// Initialize data
		// If a god UUID already exists, set it
		let mut connection = app.db_pool.begin().await?;

		let god_uuid = db::get_god_user_id(&mut connection).await?;
		if let Some(uuid) = god_uuid {
			rbac::GOD_USER_ID
				.set(uuid)
				.expect("GOD_USER_ID was already set");
		}

		let resource_types =
			db::get_all_resource_types(&mut connection).await?;
		rbac::RESOURCE_TYPES
			.set(resource_types)
			.expect("RESOURCE_TYPES is already set");

		drop(connection);

		Ok(())
	}
}

async fn migrate_database(
	app: &App,
	db_version: Version,
) -> Result<(), sqlx::Error> {
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
