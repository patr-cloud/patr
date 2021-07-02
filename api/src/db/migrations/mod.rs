use semver::Version;

/// This module is used to migrate the database to updated version
use crate::{db, utils::constants, Database};

/// # Description
/// The function is used to migrate the database from the current version to a
/// version set in ['Constants`]
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `from_version` - A struct containing version of the DB, more info here:
///   [`Version`]: Version
///
/// # Return
/// This function returns Result<(), Error> containing an empty response or
/// sqlx::error
///
/// [`Constants`]: api/src/utils/constants.rs
/// [`Transaction`]: Transaction
pub async fn migrate_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	from_version: Version,
) -> Result<(), sqlx::Error> {
	let migrations = vec!["0.0.0"];
	let db_version = from_version.to_string();

	let mut migrating = false;

	for migration_version in migrations {
		if migration_version == db_version {
			migrating = true;
		}
		if !migrating {
			continue;
		}
		#[allow(clippy::single_match)]
		match migration_version {
			"0.0.0" => (),
			_ => (),
		}
	}

	db::set_database_version(connection, &constants::DATABASE_VERSION).await?;

	Ok(())
}
