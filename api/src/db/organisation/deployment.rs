use sqlx::Transaction;

use crate::{
	models::db_mapping::{Deployment, DockerRepository},
	query,
	query_as,
	Database,
};

pub async fn initialize_deployer_pre(
	transaction: &mut Transaction<'_, Database>,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing deployment tables");
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			registry VARCHAR(255) NOT NULL DEFAULT "registry.docker.vicara.co",
			image_name VARCHAR(512) NOT NULL,
			image_tag VARCHAR(255) NOT NULL,
			domain_id BINARY(16) NOT NULL,
			sub_domain VARCHAR(255) NOT NULL,
			path VARCHAR(255) NOT NULL DEFAULT "/",
			/* TODO change port to port array, and take image from docker_registry_repository */
			port SMALLINT UNSIGNED NOT NULL,
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
	transaction: &mut Transaction<'_, Database>,
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
	transaction: &mut Transaction<'_, Database>,
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
	connection: &mut Transaction<'_, Database>,
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
	connection: &mut Transaction<'_, Database>,
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
	connection: &mut Transaction<'_, Database>,
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
	connection: &mut Transaction<'_, Database>,
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

pub async fn create_deployment(
	connection: &mut Transaction<'_, Database>,
	deployment_id: &[u8],
	name: &str,
	registry: &str,
	image_name: &str,
	image_tag: &str,
	domain_id: &[u8],
	sub_domain: &str,
	path: &str,
	port: u16,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment
		VALUES
			(?, ?, ?, ?, ?, ?, ?, ?, ?);
		"#,
		deployment_id,
		name,
		registry,
		image_name,
		image_tag,
		domain_id,
		sub_domain,
		path,
		port
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn get_deployments_by_image_name_and_tag_for_organisation(
	connection: &mut Transaction<'_, Database>,
	image_name: &str,
	image_tag: &str,
	organisation_id: &[u8],
) -> Result<Vec<Deployment>, sqlx::Error> {
	query_as!(
		Deployment,
		r#"
		SELECT
			deployment.*
		FROM
			deployment,
            resource
		WHERE
            deployment.id = resource.id AND
			image_name = ? AND
			image_tag = ? AND
            resource.owner_id = ?;
		"#,
		image_name,
		image_tag,
		organisation_id
	)
	.fetch_all(connection)
	.await
}

pub async fn get_deployments_for_organisation(
	connection: &mut Transaction<'_, Database>,
	organisation_id: &[u8],
) -> Result<Vec<Deployment>, sqlx::Error> {
	query_as!(
		Deployment,
		r#"
		SELECT
			deployment.*
		FROM
			deployment,
			resource
		WHERE
			resource.id = deployment.id AND
			resource.owner_id = ?;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await
}

pub async fn get_deployment_by_id(
	connection: &mut Transaction<'_, Database>,
	deployment_id: &[u8],
) -> Result<Option<Deployment>, sqlx::Error> {
	Ok(query_as!(
		Deployment,
		r#"
			SELECT
				*
			FROM
				deployment
			WHERE
				id = ?;
			"#,
		deployment_id
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.next())
}

pub async fn get_deployment_by_entry_point(
	connection: &mut Transaction<'_, Database>,
	domain_id: &[u8],
	sub_domain: &str,
	path: &str,
) -> Result<Option<Deployment>, sqlx::Error> {
	Ok(query_as!(
		Deployment,
		r#"
			SELECT
				*
			FROM
				deployment
			WHERE
				domain_id = ? AND
				sub_domain = ? AND
				path = ?;
			"#,
		domain_id,
		sub_domain,
		path
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.next())
}

pub async fn delete_deployment_by_id(
	connection: &mut Transaction<'_, Database>,
	deployment_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment
		WHERE
			id = ?;
		"#,
		deployment_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}
