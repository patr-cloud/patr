use std::cmp::Ordering;

use semver::Version;

use crate::prelude::*;

/// Initializes the database, and performs migrations if necessary
#[instrument(skip(app))]
pub async fn initialize(app: &AppState) -> Result<(), ErrorType> {
	info!("Initializing database");

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

	query!(
		r#"
		CREATE EXTENSION IF NOT EXISTS btree_gist;
		"#
	)
	.execute(&app.database)
	.await?;

	// If no tables exist in the database, initialize fresh
	if tables.is_empty() {
		warn!("No tables exist. Creating fresh");

		// Create all tables
		super::initialize_meta_tables(&mut transaction).await?;
		super::initialize_user_tables(&mut transaction).await?;
		super::initialize_workspace_tables(&mut transaction).await?;
		super::initialize_rbac_tables(&mut transaction).await?;

		super::initialize_meta_indices(&mut transaction).await?;
		super::initialize_user_indices(&mut transaction).await?;
		super::initialize_workspace_indices(&mut transaction).await?;
		super::initialize_rbac_indices(&mut transaction).await?;

		super::initialize_meta_constraints(&mut transaction).await?;
		super::initialize_user_constraints(&mut transaction).await?;
		super::initialize_workspace_constraints(&mut transaction).await?;
		super::initialize_rbac_constraints(&mut transaction).await?;

		// Set the database schema version
		query!(
			r#"
			INSERT INTO
				meta_data(
					id,
					value
				)
			VALUES
				('version_major', $1),
				('version_minor', $2),
				('version_patch', $3)
			ON CONFLICT(id) DO UPDATE SET
				value = EXCLUDED.value;
			"#,
			constants::DATABASE_VERSION.major.to_string(),
			constants::DATABASE_VERSION.minor.to_string(),
			constants::DATABASE_VERSION.patch.to_string()
		)
		.execute(&mut *transaction)
		.await?;

		transaction.commit().await?;

		info!("Database created fresh");

		Ok(())
	} else {
		// If it already exists, perform a migration with the known values

		let rows = query!(
			r#"
			SELECT
				*
			FROM
				meta_data
			WHERE
				id = 'version_major' OR
				id = 'version_minor' OR
				id = 'version_patch';
			"#
		)
		.fetch_all(&mut *transaction)
		.await?;
		let mut version = Version::new(0, 0, 0);

		// If versions can't be parsed, assume it to be the max value, so that
		// migrations would fail
		for row in rows {
			match row.id.as_str() {
				"version_major" => {
					version.major = row.value.parse::<u64>().unwrap_or(u64::MAX);
				}
				"version_minor" => {
					version.minor = row.value.parse::<u64>().unwrap_or(u64::MAX);
				}
				"version_patch" => {
					version.patch = row.value.parse::<u64>().unwrap_or(u64::MAX);
				}
				_ => {}
			}
		}

		match version.cmp(&constants::DATABASE_VERSION) {
			Ordering::Greater => {
				error!("Database version is higher than what's recognised. Exiting...");
				panic!();
			}
			Ordering::Less => {
				info!(
					"Migrating from {}.{}.{}",
					version.major, version.minor, version.patch
				);

				// migrations::run_migrations(&mut transaction, version, &app.config).await?;

				transaction.commit().await?;
				info!(
					"Migration completed. Database is now at version {}.{}.{}",
					constants::DATABASE_VERSION.major,
					constants::DATABASE_VERSION.minor,
					constants::DATABASE_VERSION.patch
				);
				transaction = app.database.begin().await?;
			}
			Ordering::Equal => {
				info!("Database already in the latest version. No migration required.");
			}
		}

		// Any initialization that needs to be done after the migration goes here:
		// ...

		drop(transaction);

		Ok(())
	}
}
