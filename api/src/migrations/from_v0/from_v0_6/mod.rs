use semver::Version;

use crate::{utils::settings::Settings, Database};

mod from_v0_6_0;
mod from_v0_6_1;
mod from_v0_6_2;

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
) -> Result<(), sqlx::Error> {
	match (version.major, version.minor, version.patch) {
		(0, 6, 0) => from_v0_6_0::migrate(&mut *connection, config).await,
		(0, 6, 1) => from_v0_6_1::migrate(&mut *connection, config).await,
		(0, 6, 2) => from_v0_6_2::migrate(&mut *connection, config).await,
		_ => {
			panic!("Migration from version {} is not implemented yet!", version)
		}
	}
}

/// # Description
/// The function is used to get a list of all 0.6.x migrations to migrate the
/// database from
///
/// # Return
/// This function returns [&'static str; _] containing a list of all migration
/// versions
pub fn get_migrations() -> Vec<&'static str> {
	vec!["0.6.0", "0.6.1"]
}
