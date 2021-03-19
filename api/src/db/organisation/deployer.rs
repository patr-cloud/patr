use crate::{models::db_mapping::DockerRepository, query, query_as};
use sqlx::{MySql, Transaction};

pub async fn initialize_deployer_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing deployer tables");
	query!(
		r#"
        CREATE TABLE IF NOT EXISTS deployment (
            id BINARY(16) PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            image_name VARCHAR(512) NOT NULL,
            image_tag VARCHAR(255) NOT NULL,
            domain_id BINARY(16) NOT NULL,
            sub_domain VARCHAR(255) NOT NULL,
            path VARCHAR(255) NOT NULL DEFAULT "/",
            UNIQUE(domain_id, sub_domain, path)
        );
        "#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS docker_registry_repository (
			id BINARY(16) PRIMARY KEY,
			organisation_id BINARY(16) NOT NULL,
			name VARCHAR(255) NOT NULL,
			UNIQUE(organisation_id, name)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_deployer_post(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT 
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_registry_repository
		ADD CONSTRAINT
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

// function to add new repositorys

pub async fn create_repository(
	transaction: &mut Transaction<'_, MySql>,
	resource_id: &[u8],
	name: &str,
	organisation_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			docker_registry_repository
		VALUES
			(?,?,?)
		"#,
		resource_id,
		organisation_id,
		name
	)
	.execute(&mut *transaction)
	.await?;
	Ok(())
}

pub async fn get_repository_by_name(
	connection: &mut Transaction<'_, MySql>,
	repository_name: &str,
	organisation_id: &[u8],
) -> Result<Option<DockerRepository>, sqlx::Error> {
	let rows = query_as!(
		DockerRepository,
		r#"
		SELECT
			*
		FROM
			docker_registry_repository
		WHERE
			name = ?
		AND
			organisation_id = ?
		"#,
		&repository_name,
		&organisation_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
}
