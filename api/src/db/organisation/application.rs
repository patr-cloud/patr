use crate::query;

use sqlx::{MySql, Transaction};
use crate::{
	models::db_mapping::{
        Application, 
        Version
    }, 
    query_as
};


pub async fn initialize_application_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing application tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS application (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(100) UNIQUE NOT NULL
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS application_version (
			application_id BINARY(16) NOT NULL,
			version VARCHAR(32) NOT NULL,
			PRIMARY KEY(application_id, version),
			FOREIGN KEY(application_id) REFERENCES application(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS application_version_platform (
			application_id BINARY(16) NOT NULL,
			version VARCHAR(32) NOT NULL,
			platform VARCHAR(60) NOT NULL,
			PRIMARY KEY(application_id, version, platform),
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
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE application
		ADD CONSTRAINT
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}


/// function to fetch all the application names.
pub async fn get_applications_for_organisation(
	connection : &mut Transaction<'_, MySql>,
	organisation_id : &[u8],
) -> Result<Vec<Application>, sqlx::Error> {
    let rows = query_as!(
		Application,
		r#"
			SELECT 
				application.*
			FROM
				application, resource
			WHERE
				resource.owner_id = ? AND
				resource.id = domain.id;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await?;

    Ok(rows)
}

/// add function to get application for specific given id
pub async fn get_application_by_id (
    connection : &mut Transaction<'_, MySql>,
    application_id : &[u8],
) -> Result<Option<Application>, sqlx::Error> {
    let rows = query_as!(
        Application,
        r#"
        SELECT 
            *
        FROM
            application
        WHERE 
            id = ?
        "#,
        application_id
    )
    .fetch_all(connection)
    .await?;

    Ok(rows.into_iter().next())
}

/// query to fetch versions for an application.
/// this query checks versions for an application from TABLE application_versions.
pub async fn get_all_versions_for_application (
    connection : &mut Transaction<'_, MySql>,
    appliction_id : &[u8],
) -> Result<Vec<Version>, sqlx::Error> {
    let versions = query_as!(
        Version,
        r#"
        SELECT 
            application_id,
            version
        FROM 
            application_version
        WHERE
            application_id = ?
        "#,
        appliction_id
    )
    .fetch_all(connection)
    .await?;

    Ok(versions)
}