use semver::Version;

mod from_v0;

/// This module is used to migrate the database to updated version
use crate::{
	db,
	utils::{constants, settings::Settings},
	Database,
};

/// # Description
/// The function is used to migrate the database from the current version to a
/// version set in ['Constants`]
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `current_db_version` - A struct containing version of the DB, more info
///   here: [`Version`]: Version
///
/// # Return
/// This function returns Result<(), Error> containing an empty response or
/// sqlx::error
///
/// [`Constants`]: api/src/utils/constants.rs
/// [`Transaction`]: Transaction
pub async fn run_migrations(
	connection: &mut <Database as sqlx::Database>::Connection,
	current_db_version: Version,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	// Take a list of migrations available in the code.
	// Skip elements on the list until your current version is the same as the
	// migrating version
	// Then start migrating versions one by one until the end
	let migrations_from = get_migrations()
		.into_iter()
		.map(|version| {
			Version::parse(version).expect("unable to parse version")
		})
		.skip_while(|version| version != &current_db_version);

	for version in migrations_from {
		match (version.major, version.minor, version.patch) {
			(0, ..) => {
				from_v0::migrate(&mut *connection, version, config).await?
			}
			_ => panic!(
				"Migration from version {} is not implemented yet!",
				version
			),
		}
	}

	db::set_database_version(connection, &constants::DATABASE_VERSION).await?;

	Ok(())
}

/// # Description
/// The function is used to get a list of all migrations to migrate the database
/// from
///
/// # Return
/// This function returns [&'static str; _] containing a list of all migration
/// versions
fn get_migrations() -> Vec<&'static str> {
	vec![
		from_v0::get_migrations(),
		// from_v1::get_migrations(),
	]
	.into_iter()
	.flatten()
	.collect()
}
