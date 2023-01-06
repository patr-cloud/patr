use semver::Version;

use crate::{
	utils::{settings::Settings, Error},
	Database,
};

mod from_v0_10;
mod from_v0_11;
mod from_v0_12;
mod from_v0_13;
mod from_v0_3;
mod from_v0_4;
mod from_v0_5;
mod from_v0_6;
mod from_v0_7;
mod from_v0_8;
mod from_v0_9;

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
		(0, 3, _) => {
			from_v0_3::migrate(&mut *connection, version, config).await
		}
		(0, 4, _) => {
			from_v0_4::migrate(&mut *connection, version, config).await
		}
		(0, 5, _) => {
			from_v0_5::migrate(&mut *connection, version, config).await
		}
		(0, 6, _) => {
			from_v0_6::migrate(&mut *connection, version, config).await
		}
		(0, 7, _) => {
			from_v0_7::migrate(&mut *connection, version, config).await
		}
		(0, 8, _) => {
			from_v0_8::migrate(&mut *connection, version, config).await
		}
		(0, 9, _) => {
			from_v0_9::migrate(&mut *connection, version, config).await
		}
		(0, 10, _) => {
			from_v0_10::migrate(&mut *connection, version, config).await
		}
		(0, 11, _) => {
			from_v0_11::migrate(&mut *connection, version, config).await
		}
		(0, 12, _) => {
			from_v0_12::migrate(&mut *connection, version, config).await
		}
		(0, 13, _) => {
			from_v0_13::migrate(&mut *connection, version, config).await
		}
		_ => {
			panic!("Migration from version {} is not implemented yet!", version)
		}
	}
}

/// # Description
/// The function is used to get a list of all 0.x.x migrations to migrate the
/// database from
///
/// # Return
/// This function returns [&'static str; _] containing a list of all migration
/// versions
pub fn get_migrations() -> Vec<&'static str> {
	vec![
		from_v0_3::get_migrations(),
		from_v0_4::get_migrations(),
		from_v0_5::get_migrations(),
		from_v0_6::get_migrations(),
		from_v0_7::get_migrations(),
		from_v0_8::get_migrations(),
		from_v0_9::get_migrations(),
		from_v0_10::get_migrations(),
		from_v0_11::get_migrations(),
		from_v0_12::get_migrations(),
		from_v0_13::get_migrations(),
	]
	.into_iter()
	.flatten()
	.collect()
}
