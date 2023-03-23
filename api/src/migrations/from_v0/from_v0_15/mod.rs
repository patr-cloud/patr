use semver::Version;

use crate::{
	utils::{settings::Settings, Error},
	Database,
};

mod from_v0_15_0;
mod from_v0_15_1;
mod from_v0_15_2;

/// # Description
/// The function is used to migrate the database from one version to another
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `version` - A struct containing the version to upgrade from. Panics if the
///   version is not 0.15.x, more info here: [`Version`]: Version
///
/// # Return
/// This function returns Result<(), Error> containing an empty response or
/// sqlx::error
///
/// [`Constants`]: api/src/utils/constants.rs
/// [`Transaction`]: Transaction
pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	version: Version,
	config: &Settings,
) -> Result<(), Error> {
	match (version.major, version.minor, version.patch) {
		(0, 15, 0) => from_v0_15_0::migrate(&mut *connection, config).await,
		(0, 15, 1) => from_v0_15_1::migrate(&mut *connection, config).await,
		(0, 15, 2) => from_v0_15_2::migrate(&mut *connection, config).await,
		_ => {
			panic!("Migration from version {} is not implemented yet!", version)
		}
	}
}

/// # Description
/// The function is used to get a list of all 0.15.x migrations to migrate the
/// database from
///
/// # Return
/// This function returns [&'static str; _] containing a list of all migration
/// versions
pub fn get_migrations() -> Vec<&'static str> {
	vec!["0.15.0", "0.15.1", "0.15.2"]
}
