use semver::Version;
use sqlx::Transaction;

use crate::{db, utils::constants, Database};

pub async fn migrate_database(
	connection: &mut Transaction<'_, Database>,
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
