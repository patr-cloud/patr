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
		CREATE TABLE deployment(
			id BYTEA CONSTRAINT deployment_pk PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			registry VARCHAR(255) NOT NULL DEFAULT 'registry.docker.vicara.co',
			image_name VARCHAR(512) NOT NULL,
			image_tag VARCHAR(255) NOT NULL,
			domain_id BYTEA NOT NULL
				CONSTRAINT deployment_fk_domain_id REFERENCES domain(id),
			sub_domain VARCHAR(255) NOT NULL,
			path VARCHAR(255) NOT NULL DEFAULT '/',
			port INTEGER NOT NULL
				CONSTRAINT deployment_chk_port_u16
					CHECK(port >= 0 AND port <= 65534),
			CONSTRAINT deployment_uq_domain_id_sub_domain_path
				UNIQUE(domain_id, sub_domain, path)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE INDEX
			deployment_idx_name
		ON
			deployment
		(name);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE INDEX
			deployment_idx_image_name_image_tag
		ON
			deployment
		(image_name, image_tag);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE INDEX
			deployment_idx_registry_image_name_image_tag
		ON
			deployment
		(registry, image_name, image_tag);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE docker_registry_repository(
			id BYTEA CONSTRAINT docker_registry_repository_pk PRIMARY KEY,
			organisation_id BYTEA NOT NULL,
			name VARCHAR(255) NOT NULL,
			CONSTRAINT docker_registry_repository_uq_organisation_id_name
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
		ADD CONSTRAINT deployment_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_registry_repository
		ADD CONSTRAINT docker_registry_repository_fk_id
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
			($1, $2, $3);
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
			name = $1
		AND
			organisation_id = $2;
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
			organisation_id = $1;
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
			id = $1;
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
			id = $1;
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
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				$6,
				$7,
				$8,
				$9
			);
		"#,
		deployment_id,
		name,
		registry,
		image_name,
		image_tag,
		domain_id,
		sub_domain,
		path,
		port as i32
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
	let rows = query!(
		r#"
		SELECT
			deployment.*
		FROM
			deployment,
			resource
		WHERE
			deployment.id = resource.id AND
			image_name = $1 AND
			image_tag = $2 AND
			resource.owner_id = $3;
		"#,
		image_name,
		image_tag,
		organisation_id
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.map(|row| Deployment {
		id: row.id,
		name: row.name,
		registry: row.registry,
		image_name: row.image_name,
		image_tag: row.image_tag,
		domain_id: row.domain_id,
		sub_domain: row.sub_domain,
		path: row.path,
		port: row.port as u16,
	})
	.collect();

	Ok(rows)
}

pub async fn get_deployments_for_organisation(
	connection: &mut Transaction<'_, Database>,
	organisation_id: &[u8],
) -> Result<Vec<Deployment>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			deployment.*
		FROM
			deployment,
			resource
		WHERE
			resource.id = deployment.id AND
			resource.owner_id = $1;
		"#,
		organisation_id
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.map(|row| Deployment {
		id: row.id,
		name: row.name,
		registry: row.registry,
		image_name: row.image_name,
		image_tag: row.image_tag,
		domain_id: row.domain_id,
		sub_domain: row.sub_domain,
		path: row.path,
		port: row.port as u16,
	})
	.collect();

	Ok(rows)
}

pub async fn get_deployment_by_id(
	connection: &mut Transaction<'_, Database>,
	deployment_id: &[u8],
) -> Result<Option<Deployment>, sqlx::Error> {
	let mut rows = query!(
		r#"
		SELECT
			*
		FROM
			deployment
		WHERE
			id = $1;
		"#,
		deployment_id
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.map(|row| Deployment {
		id: row.id,
		name: row.name,
		registry: row.registry,
		image_name: row.image_name,
		image_tag: row.image_tag,
		domain_id: row.domain_id,
		sub_domain: row.sub_domain,
		path: row.path,
		port: row.port as u16,
	});

	Ok(rows.next())
}

pub async fn get_deployment_by_entry_point(
	connection: &mut Transaction<'_, Database>,
	domain_id: &[u8],
	sub_domain: &str,
	path: &str,
) -> Result<Option<Deployment>, sqlx::Error> {
	let mut rows = query!(
		r#"
		SELECT
			*
		FROM
			deployment
		WHERE
			domain_id = $1 AND
			sub_domain = $2 AND
			path = $3;
		"#,
		domain_id,
		sub_domain,
		path
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.map(|row| Deployment {
		id: row.id,
		name: row.name,
		registry: row.registry,
		image_name: row.image_name,
		image_tag: row.image_tag,
		domain_id: row.domain_id,
		sub_domain: row.sub_domain,
		path: row.path,
		port: row.port as u16,
	});

	Ok(rows.next())
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
			id = $1;
		"#,
		deployment_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}
