use semver::Version;

use crate::{app::App, query, Database};

pub async fn initialize_meta_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing meta tables");
	query!(
		r#"
		CREATE TABLE meta_data(
			id VARCHAR(100) CONSTRAINT meta_data_pk PRIMARY KEY,
			value TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

pub async fn initialize_meta_post(
	_connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up meta tables initialization");
	Ok(())
}

pub async fn set_database_version(
	connection: &mut <Database as sqlx::Database>::Connection,
	version: &Version,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			meta_data (id, value)
		VALUES
			('version_major', $1),
			('version_minor', $2),
			('version_patch', $3)
		ON CONFLICT(id) DO UPDATE SET
			value = EXCLUDED.value;
		"#,
		version.major.to_string(),
		version.minor.to_string(),
		version.patch.to_string()
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_database_version(app: &App) -> Result<Version, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			meta_data
		WHERE
			id = 'version_major' OR
			id = 'version_minor' OR
			id = 'version_patch';
		"#,
	)
	.fetch_all(&app.database)
	.await?;

	let mut version = Version::new(0, 0, 0);

	// If versions can't be parsed, assume it to be the max value, so that
	// migrations would fail
	for row in rows {
		match row.id.as_str() {
			"version_major" => {
				version.major = row.value.parse::<u64>().unwrap_or(u64::MAX);
			}
			"version_minor" => {
				version.minor = row.value.parse::<u64>().unwrap_or(u64::MAX);
			}
			"version_patch" => {
				version.patch = row.value.parse::<u64>().unwrap_or(u64::MAX);
			}
			_ => {}
		}
	}

	Ok(version)
}
