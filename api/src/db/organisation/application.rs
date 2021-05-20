use sqlx::Transaction;

use crate::{
	models::db_mapping::{Application, ApplicationVersion},
	query,
	query_as,
	Database,
};

// TODO these haven't been migrated to postgres yet
pub async fn initialize_application_pre(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing application tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS application (
			id BYTEA CONSTRAINT application_pk PRIMARY KEY,
			name VARCHAR(100) NOT NULL CONSTRAINT application_uq_name UNIQUE
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS application_version (
			application_id BYTEA NOT NULL
				CONSTRAINT application_version_fk_application_id
					REFERENCES application(id),
			version VARCHAR(32) NOT NULL,
			CONSTRAINT application_version_pk
				PRIMARY KEY(application_id, version)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS application_version_platform (
			application_id BYTEA NOT NULL,
			version VARCHAR(32) NOT NULL,
			platform VARCHAR(60) NOT NULL,
			CONSTRAINT application_version_platform_pk
				PRIMARY KEY(application_id, version, platform),
			CONSTRAINT application_version_platform_fk_application_id_version
				FOREIGN KEY(application_id, version)
					REFERENCES application_version(application_id, version)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_application_post(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE application
		ADD CONSTRAINT application_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

/// function to fetch all the application names.
pub async fn get_applications_in_organisation(
	connection: &mut Transaction<'_, Database>,
	organisation_id: &[u8],
) -> Result<Vec<Application>, sqlx::Error> {
	query_as!(
		Application,
		r#"
		SELECT
			application.*
		FROM
			application
		INNER JOIN
			resource
		ON
			application.id = resource.id
		WHERE
			resource.owner_id = $1;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await
}

/// add function to get application for specific given id
pub async fn get_application_by_id(
	connection: &mut Transaction<'_, Database>,
	application_id: &[u8],
) -> Result<Option<Application>, sqlx::Error> {
	let rows = query_as!(
		Application,
		r#"
		SELECT
			*
		FROM
			application
		WHERE
			id = $1;
		"#,
		application_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}

/// query to fetch versions for an application.
/// this query checks versions for an application from TABLE
/// application_versions.
pub async fn get_all_versions_for_application(
	connection: &mut Transaction<'_, Database>,
	appliction_id: &[u8],
) -> Result<Vec<ApplicationVersion>, sqlx::Error> {
	let versions = query_as!(
		ApplicationVersion,
		r#"
		SELECT
			application_id,
			version
		FROM
			application_version
		WHERE
			application_id = $1;
		"#,
		appliction_id
	)
	.fetch_all(connection)
	.await?;

	Ok(versions)
}
