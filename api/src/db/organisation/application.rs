use crate::query;

use sqlx::{MySql, Transaction};
use crate::{models::db_mapping::
    {
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


// function to fetch all the application names.
//TODO: implement the function
pub async fn get_applications_for_organisation(
	connection : &mut Transaction<'_, MySql>,
	organisation_id : &[u8],
) -> Result<Vec<Application>, sqlx::Error> {
	// sql query to fetch application names.
    
    // let rows : Vec<Application> = Vec::new();
    let rows = query_as!(
		Application,
		r#"
			SELECT 
				application.id,
				application.name
			FROM
				application
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

// add function to get application for specific given id
// TODO: implement this function

pub async fn get_application_by_id (
    connection : &mut Transaction<'_, MySql>,
    application_id : &[u8],
) -> Result<Option<Application>, sqlx::Error> {
    let rows = query_as!(
        Application,
        r#"
        SELECT 
            id,
            name
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
    // Ok(None)
}

// query to fetch versions for an application.
// check table application_versions
pub async fn get_versions_for_application(
    connection : &mut Transaction<'_, MySql>,
    appliction_id : &[u8],
) -> Result<Option<Version>, sqlx::Error> {
    
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

    Ok(versions.into_iter().next())
}