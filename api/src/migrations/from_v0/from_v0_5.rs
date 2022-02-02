use semver::Version;

use crate::{utils::settings::Settings, Database};

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
		(0, 5, 0) => migrate_from_v0_5_0(&mut *connection, config).await?,
		_ => {
			panic!("Migration from version {} is not implemented yet!", version)
		}
	}

	Ok(())
}

async fn migrate_from_v0_5_0(
	_connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	Ok(())
}
