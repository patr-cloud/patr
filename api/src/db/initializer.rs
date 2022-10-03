use std::cmp::Ordering;

use api_models::utils::Uuid;
use chrono::{Datelike, Utc};

use crate::{
	app::App,
	db::{self, get_database_version, set_database_version},
	migrations,
	models::{deployment, rabbitmq::WorkspaceRequestData, rbac},
	query,
	service,
	utils::{constants, Error},
};

pub async fn initialize(app: &App) -> Result<(), Error> {
	log::info!("Initializing database");

	let tables = query!(
		r#"
		SELECT
			*
		FROM
			information_schema.tables
		WHERE
			table_catalog = $1 AND
			table_schema = 'public' AND
			table_type = 'BASE TABLE' AND
			table_name != 'spatial_ref_sys';
		"#,
		app.config.database.database
	)
	.fetch_all(&app.database)
	.await?;
	let mut transaction = app.database.begin().await?;

	query!(
		r#"
		CREATE EXTENSION IF NOT EXISTS postgis;
		"#
	)
	.execute(&app.database)
	.await?;

	query!(
		r#"
		CREATE EXTENSION IF NOT EXISTS citext;
		"#
	)
	.execute(&app.database)
	.await?;

	// If no tables exist in the database, initialize fresh
	if tables.is_empty() {
		log::warn!("No tables exist. Creating fresh");

		// Create all tables
		db::initialize_meta_pre(&mut transaction).await?;
		db::initialize_users_pre(&mut transaction).await?;
		db::initialize_workspaces_pre(&mut transaction).await?;
		db::initialize_rbac_pre(&mut transaction).await?;

		db::initialize_rbac_post(&mut transaction).await?;
		db::initialize_workspaces_post(&mut transaction).await?;
		db::initialize_users_post(&mut transaction).await?;
		db::initialize_meta_post(&mut transaction).await?;

		// Set the database schema version
		set_database_version(&mut transaction, &constants::DATABASE_VERSION)
			.await?;

		// Queue the payment for the current month
		let now = Utc::now();
		let month = now.month();
		let year = now.year();
		let request_id = Uuid::new_v4();
		service::send_message_to_billing_queue(
			&WorkspaceRequestData::ProcessWorkspaces {
				month: if month == 12 { 1 } else { month + 1 },
				year: if month == 12 { year + 1 } else { year },
				request_id: request_id.clone(),
			},
			&app.config,
			&request_id,
		)
		.await?;

		transaction.commit().await?;

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

				migrations::run_migrations(
					&mut transaction,
					version,
					&app.config,
				)
				.await?;

				transaction.commit().await?;
				log::info!(
					"Migration completed. Database is now at version {}.{}.{}",
					constants::DATABASE_VERSION.major,
					constants::DATABASE_VERSION.minor,
					constants::DATABASE_VERSION.patch
				);
				transaction = app.database.begin().await?;
			}
			Ordering::Equal => {
				log::info!("Database already in the latest version. No migration required.");
			}
		}

		// Initialize data
		// If a god UUID already exists, set it

		let god_uuid = db::get_god_user_id(&mut transaction).await?;
		if let Some(uuid) = god_uuid {
			rbac::GOD_USER_ID
				.set(uuid)
				.expect("GOD_USER_ID was already set");
		}

		let resource_types = db::get_all_resource_types(&mut transaction)
			.await?
			.into_iter()
			.map(|resource_type| (resource_type.name, resource_type.id))
			.collect();
		rbac::RESOURCE_TYPES
			.set(resource_types)
			.expect("RESOURCE_TYPES is already set");

		let permissions = db::get_all_permissions(&mut transaction)
			.await?
			.into_iter()
			.map(|permission| (permission.name, permission.id))
			.collect();
		rbac::PERMISSIONS
			.set(permissions)
			.expect("PERMISSIONS is already set");

		let machine_types =
			db::get_all_deployment_machine_types(&mut transaction)
				.await?
				.into_iter()
				.map(|machine_type| {
					(
						machine_type.id,
						(machine_type.cpu_count, machine_type.memory_count),
					)
				})
				.collect();
		deployment::MACHINE_TYPES
			.set(machine_types)
			.expect("MACHINE_TYPES is already set");

		drop(transaction);

		Ok(())
	}
}
