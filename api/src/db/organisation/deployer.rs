use crate::{
	models::db_mapping::{Deployment, DockerRepository},
	query,
	query_as,
};
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
			registry ENUM("registry.hub.docker.com", "registry.vicara.co") NOT NULL DEFAULT "registry.vicara.co",
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
			(?,?,?);
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
			organisation_id = ?;
		"#,
		repository_name,
		organisation_id
	)
	.fetch_all(connection)
	.await?;

	Ok(rows.into_iter().next())
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

pub async fn get_deployment_by_image_name_and_tag(
	connection: &mut Transaction<'_, MySql>,
	image_name: &str,
	tag: &str,
) -> Result<Vec<Deployment>, sqlx::Error> {
	let rows = query_as!(
		Deployment,
		r#"
		SELECT
			*
		FROM
			deployment
		WHERE
			image_name = ? AND
			image_tag = ?;
		"#,
		image_name,
		tag
	)
	.fetch_all(connection)
	.await?;

	Ok(rows)
}
