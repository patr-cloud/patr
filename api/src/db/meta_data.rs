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
			meta_data
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
	.await?;
	Ok(())
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

	for row in rows {
		match (row.id.as_str(), row.value.parse()) {
			("version_major", Ok(value)) => {
				version.major = value;
			}
			("version_minor", Ok(value)) => {
				version.minor = value;
			}
			("version_patch", Ok(value)) => {
				version.patch = value;
			}
			_ => {}
		}
	}

	Ok(version)
}
