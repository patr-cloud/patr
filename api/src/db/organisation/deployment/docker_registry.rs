use sqlx::{MySql, Transaction};

use crate::{models::db_mapping::DockerRepository, query, query_as};

pub async fn initialize_docker_registry_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing docker registry tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS docker_registry_repository (
			id BINARY(16) PRIMARY KEY,
			organisation_id BINARY(16) NOT NULL,
			name VARCHAR(255) NOT NULL,
			UNIQUE(organisation_id, name),
			FOREIGN KEY(organisation_id) REFERENCES organisation(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_docker_registry_post(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up docker registry tables initialization");
	query!(
		r#"
		ALTER TABLE docker_registry_repository
		ADD CONSTRAINT
		FOREIGN KEY (id, organisation_id)
		REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

// function to add new repositorys
pub async fn create_docker_repository(
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
			(?, ?, ?);
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
	query_as!(
		DockerRepository,
		r#"
		SELECT
			*
		FROM
			docker_registry_repository
		WHERE
			name = ?
		AND
			organisation_id = ?;
		"#,
		repository_name,
		organisation_id
	)
	.fetch_all(connection)
	.await
	.map(|rows| rows.into_iter().next())
}

pub async fn get_docker_repositories_for_organisation(
	connection: &mut Transaction<'_, MySql>,
	organisation_id: &[u8],
) -> Result<Vec<DockerRepository>, sqlx::Error> {
	query_as!(
		DockerRepository,
		r#"
		SELECT
			*
		FROM
			docker_registry_repository
		WHERE
			organisation_id = ?;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await
}

pub async fn get_docker_repository_by_id(
	connection: &mut Transaction<'_, MySql>,
	repository_id: &[u8],
) -> Result<Option<DockerRepository>, sqlx::Error> {
	query_as!(
		DockerRepository,
		r#"
		SELECT
			*
		FROM
			docker_registry_repository
		WHERE
			id = ?;
		"#,
		repository_id
	)
	.fetch_all(connection)
	.await
	.map(|repos| repos.into_iter().next())
}

pub async fn delete_docker_repository_by_id(
	connection: &mut Transaction<'_, MySql>,
	repository_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			docker_registry_repository
		WHERE
			id = ?;
		"#,
		repository_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}
