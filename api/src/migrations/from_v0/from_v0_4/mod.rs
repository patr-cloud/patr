use semver::Version;

use crate::{
	utils::{settings::Settings, Error},
	Database,
};

mod from_v0_4_0;
mod from_v0_4_1;
mod from_v0_4_2;
mod from_v0_4_3;
mod from_v0_4_4;
mod from_v0_4_5;
mod from_v0_4_6;
mod from_v0_4_7;
mod from_v0_4_8;
mod from_v0_4_9;

/// # Description
/// The function is used to migrate the database from one version to another
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `version` - A struct containing the version to upgrade from. Panics if the
///   version is not 0.x.x, more info here: [`Version`]: Version
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
		(0, 4, 0) => from_v0_4_0::migrate(&mut *connection, config).await,
		(0, 4, 1) => from_v0_4_1::migrate(&mut *connection, config).await,
		(0, 4, 2) => from_v0_4_2::migrate(&mut *connection, config).await,
		(0, 4, 3) => from_v0_4_3::migrate(&mut *connection, config).await,
		(0, 4, 4) => from_v0_4_4::migrate(&mut *connection, config).await,
		(0, 4, 5) => from_v0_4_5::migrate(&mut *connection, config).await,
		(0, 4, 6) => from_v0_4_6::migrate(&mut *connection, config).await,
		(0, 4, 7) => from_v0_4_7::migrate(&mut *connection, config).await,
		(0, 4, 8) => from_v0_4_8::migrate(&mut *connection, config).await,
		(0, 4, 9) => from_v0_4_9::migrate(&mut *connection, config).await,
		_ => {
			panic!("Migration from version {} is not implemented yet!", version)
		}
	}
}

/// # Description
/// The function is used to get a list of all 0.4.x migrations to migrate the
/// database from
///
/// # Return
/// This function returns [&'static str; _] containing a list of all migration
/// versions
pub fn get_migrations() -> Vec<&'static str> {
	vec![
		"0.4.0", "0.4.1", "0.4.2", "0.4.3", "0.4.4", "0.4.5", "0.4.6", "0.4.7",
		"0.4.8", "0.4.9",
	]
}
