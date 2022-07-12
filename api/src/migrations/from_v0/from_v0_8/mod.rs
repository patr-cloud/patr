use semver::Version;

use crate::{
	utils::{settings::Settings, Error},
	Database,
};

mod from_v0_8_0;
mod from_v0_8_1;
mod from_v0_8_2;
mod from_v0_8_3;
mod from_v0_8_4;
mod from_v0_8_5;
mod from_v0_8_6;

/// # Description
/// The function is used to migrate the database from one version to another
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `version` - A struct containing the version to upgrade from. Panics if the
///   version is not 0.8.x, more info here: [`Version`]: Version
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
		(0, 8, 0) => from_v0_8_0::migrate(&mut *connection, config).await,
		(0, 8, 1) => from_v0_8_1::migrate(&mut *connection, config).await,
		(0, 8, 2) => from_v0_8_2::migrate(&mut *connection, config).await,
		(0, 8, 3) => from_v0_8_3::migrate(&mut *connection, config).await,
		(0, 8, 4) => from_v0_8_4::migrate(&mut *connection, config).await,
		(0, 8, 5) => from_v0_8_5::migrate(&mut *connection, config).await,
		(0, 8, 6) => from_v0_8_6::migrate(&mut *connection, config).await,
		_ => {
			panic!("Migration from version {} is not implemented yet!", version)
		}
	}
}

/// # Description
/// The function is used to get a list of all 0.8.x migrations to migrate the
/// database from
///
/// # Return
/// This function returns [&'static str; _] containing a list of all migration
/// versions
pub fn get_migrations() -> Vec<&'static str> {
	vec![
		"0.8.0", "0.8.1", "0.8.2", "0.8.3", "0.8.4", "0.8.5", "0.8.6",
	]
}
