use semver::Version;

use crate::{Database, query};

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
) -> Result<(), sqlx::Error> {
	match (version.major, version.minor, version.patch) {
		(0, 3, 0) => migrate_from_v0_3_0(&mut *connection).await?,
		_ => {
			panic!("Migration from version {} is not implemented yet!", version)
		}
	}

	Ok(())
}

/// # Description
/// The function is used to get a list of all 0.3.x migrations to migrate the
/// database from
///
/// # Return
/// This function returns [&'static str; _] containing a list of all migration
/// versions
pub fn get_migrations() -> Vec<&'static str> {
	vec![
		"0.3.0",
	]
}

async fn migrate_from_v0_3_0(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	// Add region column
	query!(
		r#"
		ALTER TABLE deployment
		ADD COLUMN region TEXT NOT NULL
		DEFAULT 'do-blr';
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
