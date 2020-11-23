use crate::app::App;

use crate::query;
use semver::Version;
use sqlx::{MySql, Transaction};

pub async fn initialize_meta_pre(
	transaction: &mut Transaction<'_, MySql>,
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

pub async fn initialize_meta_post(
	_transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	Ok(())
}

pub async fn set_database_version(
	app: &App,
	version: &Version,
) -> Result<(), sqlx::Error> {
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
	.execute(&app.mysql)
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
	.fetch_all(&app.mysql)
	.await?;

	let mut version = Version::new(0, 0, 0);

	for row in rows {
		match row.id.as_ref() {
			"version_major" => {
				version.major = row.value.parse::<u64>().unwrap()
			}
			"version_minor" => {
				version.minor = row.value.parse::<u64>().unwrap()
			}
			"version_patch" => {
				version.patch = row.value.parse::<u64>().unwrap()
			}
			_ => {}
		}
	}

	Ok(version)
}
