use semver::Version;

use crate::{
	utils::{settings::Settings, Error},
	Database,
};

mod from_v0_13_0;
mod from_v0_13_1;
mod from_v0_13_2;
mod from_v0_13_3;
mod from_v0_13_4;

/// # Description
/// The function is used to migrate the database from one version to another
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `version` - A struct containing the version to upgrade from. Panics if the
///   version is not 0.13.x, more info here: [`Version`]: Version
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
		(0, 13, 0) => from_v0_13_0::migrate(&mut *connection, config).await,
		(0, 13, 1) => from_v0_13_1::migrate(&mut *connection, config).await,
		(0, 13, 2) => from_v0_13_2::migrate(&mut *connection, config).await,
		(0, 13, 3) => from_v0_13_3::migrate(&mut *connection, config).await,
		(0, 13, 4) => from_v0_13_4::migrate(&mut *connection, config).await,
		_ => {
			panic!("Migration from version {} is not implemented yet!", version)
		}
	}
}

/// # Description
/// The function is used to get a list of all 0.13.x migrations to migrate the
/// database from
///
/// # Return
/// This function returns [&'static str; _] containing a list of all migration
/// versions
pub fn get_migrations() -> Vec<&'static str> {
	vec!["0.13.0", "0.13.1", "0.13.2", "0.13.3", "0.13.4"]
}
