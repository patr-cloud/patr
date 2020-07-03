use crate::app::App;

use crate::query;
use semver::Version;

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
			metaId = 'version_major' OR
			metaId = 'version_minor' OR
			metaId = 'version_patch';
		"#,
	)
	.fetch_all(&app.db_pool)
	.await?;

	let mut version = Version::new(0, 0, 0);

	for row in rows {
		match row.metaId.as_ref() {
			"version_major" => version.major = row.value.parse::<u64>().unwrap(),
			"version_minor" => version.minor = row.value.parse::<u64>().unwrap(),
			"version_patch" => version.patch = row.value.parse::<u64>().unwrap(),
			_ => {}
		}
	}

	Ok(version)
}
