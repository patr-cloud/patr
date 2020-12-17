// use crate::query;

// use sqlx::{MySql, Transaction};
// use crate::{models::db_mapping::Application, query_as};


// pub async fn initialize_application_pre(
// 	transaction: &mut Transaction<'_, MySql>,
// ) -> Result<(), sqlx::Error> {
// 	log::info!("Initializing application tables");
// 	query!(
// 		r#"
// 		CREATE TABLE IF NOT EXISTS application (
// 			id BINARY(16) PRIMARY KEY,
// 			name VARCHAR(100) UNIQUE NOT NULL
// 		);
// 		"#
// 	)
// 	.execute(&mut *transaction)
// 	.await?;

// 	query!(
// 		r#"
// 		CREATE TABLE IF NOT EXISTS application_version (
// 			application_id BINARY(16) NOT NULL,
// 			version VARCHAR(32) NOT NULL,
// 			PRIMARY KEY(application_id, version),
// 			FOREIGN KEY(application_id) REFERENCES application(id)
// 		);
// 		"#
// 	)
// 	.execute(&mut *transaction)
// 	.await?;

// 	query!(
// 		r#"
// 		CREATE TABLE IF NOT EXISTS application_version_platform (
// 			application_id BINARY(16) NOT NULL,
// 			version VARCHAR(32) NOT NULL,
// 			platform VARCHAR(60) NOT NULL,
// 			PRIMARY KEY(application_id, version, platform),
// 			FOREIGN KEY(application_id, version)
// 				REFERENCES application_version(application_id, version)
// 		);
// 		"#
// 	)
// 	.execute(&mut *transaction)
// 	.await?;

// 	Ok(())
// }

// pub async fn initialize_application_post(
// 	transaction: &mut Transaction<'_, MySql>,
// ) -> Result<(), sqlx::Error> {
// 	query!(
// 		r#"
// 		ALTER TABLE application
// 		ADD CONSTRAINT
// 		FOREIGN KEY(id) REFERENCES resource(id);
// 		"#
// 	)
// 	.execute(&mut *transaction)
// 	.await?;

// 	Ok(())
// }


// pub async fn get_applications_for_organisation(
// 	connection : &mut Transaction<'_, MySql>,
// 	organisation_id : &[u8],
// ) -> Result<Vec<Application>, sqlx::Error> {
// 	// sql query to fetch application names.
// 	// todo : add resource authentication
// 	let rows = query_as!(
// 		Application,
// 		r#"
// 			SELECT 
// 				application.id,
// 				application.name
// 			FROM
// 				application
// 			WHERE
// 				resource.owner_id = ? AND
// 				resource.id = domain.id;
// 		"#,
// 		organisation_id
// 	)
// 	.fetch_all(connection)
// 	.await?;

// 	Ok(rows)
// }

// // add function to get application for specific given id
