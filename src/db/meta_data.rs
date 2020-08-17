use crate::app::App;

use crate::query;
use semver::Version;
use sqlx::{pool::PoolConnection, MySqlConnection, Transaction};

pub async fn initialize_meta(
	transaction: &mut Transaction<PoolConnection<MySqlConnection>>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing meta tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS meta_data (
			id VARCHAR(100) PRIMARY KEY,
			value TEXT NOT NULL
		);
		"#
	)
	.execute(transaction)
	.await?;
	Ok(())
}

pub async fn set_database_version(app: &App, version: &Version) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			meta_data
		VALUES
			('version_major', ?),
			('version_minor', ?),
			('version_patch', ?)
		ON DUPLICATE KEY UPDATE
			value = VALUES(value);
		"#,
		version.major,
		version.minor,
		version.patch
	)
	.execute(&app.db_pool)
	.await?;
	Ok(())
}

pub async fn get_database_version(app: &App) -> Result<Version, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT * FROM
			meta_data
		WHERE
			id = 'version_major' OR
			id = 'version_minor' OR
			id = 'version_patch';
		"#,
	)
	.fetch_all(&app.db_pool)
	.await?;

	let mut version = Version::new(0, 0, 0);

	for row in rows {
		match row.id.as_ref() {
			"version_major" => version.major = row.value.parse::<u64>().unwrap(),
			"version_minor" => version.minor = row.value.parse::<u64>().unwrap(),
			"version_patch" => version.patch = row.value.parse::<u64>().unwrap(),
			_ => {}
		}
	}

	Ok(version)
}
