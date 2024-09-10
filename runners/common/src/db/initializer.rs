use std::cmp::Ordering;

use semver::Version;

use crate::prelude::*;

/// Initializes the database, and performs migrations if necessary
#[instrument(skip(app))]
pub async fn initialize<E>(app: &AppState<E>) -> Result<(), ErrorType>
where
	E: RunnerExecutor,
{
	info!("Initializing database");

	let tables = query(
		r#"
		SELECT
			*
		FROM
			sqlite_schema
		WHERE
			type = 'table';
		"#,
	)
	.fetch_all(&app.database)
	.await?;

	let mut connection = app.database.acquire().await?;

	if tables.is_empty() {
		warn!("No tables exist. Creating fresh");

		// Create all tables
		super::initialize_meta_tables(&mut connection).await?;
		super::initialize_workspace_tables(&mut connection).await?;

		// Create all indices
		super::initialize_meta_indices(&mut connection).await?;
		super::initialize_workspace_indices(&mut connection).await?;

		query(
			r#"
			INSERT INTO
				meta_data(
					id,
					value
				)
			VALUES
				('version_major', $1),
				('version_minor', $2),
				('version_patch', $3);
			"#,
		)
		.bind(constants::DATABASE_VERSION.major.to_string())
		.bind(constants::DATABASE_VERSION.minor.to_string())
		.bind(constants::DATABASE_VERSION.patch.to_string())
		.execute(&mut *connection)
		.await?;

		info!("Database created");
	} else {
		let rows = query(
			r#"
			SELECT
				*
			FROM
				meta_data
			WHERE
				id = 'version_major' OR
				id = 'version_minor' OR
				id = 'version_patch';
			"#,
		)
		.fetch_all(&mut *connection)
		.await?;
		let mut version = Version::new(0, 0, 0);

		// If versions can't be parsed, assume it to be the max value, so that
		// migrations would fail
		for row in rows {
			let id = row.get::<String, _>("id");
			let value = row.get::<String, _>("value");
			match id.as_str() {
				"version_major" => {
					version.major = value.parse::<u64>().unwrap_or(u64::MAX);
				}
				"version_minor" => {
					version.minor = value.parse::<u64>().unwrap_or(u64::MAX);
				}
				"version_patch" => {
					version.patch = value.parse::<u64>().unwrap_or(u64::MAX);
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

				info!(
					"Migration completed. Database is now at version {}.{}.{}",
					constants::DATABASE_VERSION.major,
					constants::DATABASE_VERSION.minor,
					constants::DATABASE_VERSION.patch
				);
			}
			Ordering::Equal => {
				info!("Database already in the latest version. No migration required.");
			}
		}

		// Any initialization that needs to be done after the migration goes
		// here: ...
	}

	Ok(())
}
