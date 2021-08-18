use api_macros::query;

use crate::Database;

pub async fn initialize_managed_database_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
    query!(
		r#"
		CREATE TYPE DATABASE_STATUS AS ENUM(
			'creating', /* Started the creation of database */
			'created', /* Successfully created the database */
			'running', /* Database is running successfully */
			'stopped', /* Database is stopped by the user */
			'errored', /* Database encountered errors */
			'deleted' /* Database is deled by the user   */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_database(
			id BYTEA CONSTRAINT managed_database_pk PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			status DATABASE_STATUS NOT NULL Default 'creating',
			database_id TEXT
				CONSTRAINT managed_database_uq_database_id UNIQUE,
			db_service TEXT,
			organisation_id BYTEA NOT NULL
				CONSTRAINT managed_database_fk_id
					REFERENCES organisation(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

    Ok(())

}

pub async fn initialize_deployment_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up managed_database tables initialization");
	query!(
		r#"
		ALTER TABLE managed_database 
			ADD CONSTRAINT managed_database_repository_fk_id_organisation_id
		FOREIGN KEY(id, organisation_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn create_managed_database(
	connection: &mut <Database as sqlx::Database>::Connection,
	managed_database_id: &[u8],
	name: &str,
	database_id: &str,
	db_service: &str,
	organisation_id: &[u8]
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			managed_database
		VALUES
			($1, $2, 'creating', $3, $4, $5);
		"#,
		managed_database_id,
		name,
		database_id,
		db_service,
		organisation_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}