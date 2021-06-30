use std::net::IpAddr;

use sqlx::types::ipnetwork::IpNetwork;
use uuid::Uuid;

use crate::{
	models::db_mapping::{Deployment, DeploymentRunner},
	query,
	query_as,
	utils::get_current_time_millis,
	Database,
};

pub async fn initialize_deployment_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE deployment(
			id BYTEA CONSTRAINT deployment_pk PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			registry VARCHAR(255) NOT NULL DEFAULT 'registry.docker.vicara.co',
			repository_id BYTEA CONSTRAINT deployment_fk_repository_id
				REFERENCES docker_registry_repository(id),
			image_name VARCHAR(512),
			image_tag VARCHAR(255) NOT NULL,
			deployed_image TEXT,
			CONSTRAINT deployment_chk_repository_id_is_valid CHECK(			
				(
					registry = 'registry.docker.vicara.co' AND
					image_name IS NULL AND
					repository_id IS NOT NULL
				)
				 OR
				(
					registry != 'registry.docker.vicara.co' AND
					image_name IS NOT NULL AND
					repository_id IS NULL
				)
			)	
		);
		"#
	)
	.execute(&mut *connection)
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
	.execute(&mut *connection)
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
	.execute(&mut *connection)
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
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_runner(
			id BYTEA CONSTRAINT deployment_runner_pk PRIMARY KEY,
			last_updated BIGINT NOT NULL
				CONSTRAINT deployment_runner_chk_last_updated_unsigned
					CHECK(last_updated >= 0),
			/* TODO add region here later */
			container_id BYTEA
				CONSTRAINT deployment_runner_uq_container_id UNIQUE
			/* TODO add score here later */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// TODO this doesn't work as of now. Need to figure out GiST
	query!(
		r#"
		CREATE INDEX
			deployment_runner_idx_last_updated
		ON
			deployment_runner
		USING GIST(last_updated);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_application_server(
			server_ip INET
				CONSTRAINT deployment_application_server_pk PRIMARY KEY,
			/* TODO add region here later */
			server_type TEXT NOT NULL DEFAULT 'do-server-1'
			/* TODO come up with a convention for ↑ this name */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DEPLOYMENT_RUNNER_STATUS AS ENUM(
			'alive',
			'starting',
			'shutting down',
			'dead'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_runner_deployment(
			deployment_id BYTEA
				CONSTRAINT deployment_runner_deployment_pk PRIMARY KEY,
			runner_id BYTEA NOT NULL,
			last_updated BIGINT NOT NULL
				CONSTRAINT deployment_runner_chk_last_updated_unsigned
					CHECK(last_updated >= 0),
			status DEPLOYMENT_RUNNER_STATUS NOT NULL DEFAULT 'alive'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			deployment_runner_deployment_idx_last_updated
		ON
			deployment_runner_deployment
		USING GIST(last_updated);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_deployment_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up deployment tables initialization");
	query!(
		r#"
		ALTER TABLE deployment ADD CONSTRAINT deployment_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn create_deployment_with_internal_registry(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	name: &str,
	repository_id: &[u8],
	image_tag: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment
		VALUES
			($1, $2, 'registry.docker.vicara.co', $3, NULL, $4, NULL);
		"#,
		deployment_id,
		name,
		repository_id,
		image_tag
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn create_deployment_with_external_registry(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	name: &str,
	registry: &str,
	image_name: &str,
	image_tag: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment
		VALUES
			($1, $2, $3, NULL, $4, $5, NULL);
		"#,
		deployment_id,
		name,
		registry,
		image_name,
		image_tag
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_deployments_by_image_name_and_tag_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	image_name: &str,
	image_tag: &str,
	organisation_id: &[u8],
) -> Result<Vec<Deployment>, sqlx::Error> {
	let rows = query_as!(
		Deployment,
		r#"
		SELECT
			deployment.*
		FROM
			deployment
		INNER JOIN
            resource
		ON
			deployment.id = resource.id
		WHERE
            image_name = $1 AND
			image_tag = $2 AND
            resource.owner_id = $3;
		"#,
		image_name,
		image_tag,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await?;

	Ok(rows)
}

pub async fn get_deployments_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
) -> Result<Vec<Deployment>, sqlx::Error> {
	let rows = query_as!(
		Deployment,
		r#"
		SELECT
			deployment.*
		FROM
			deployment
		INNER JOIN
			resource
		ON
			deployment.id = resource.id
		WHERE
			resource.id = deployment.id AND
			resource.owner_id = $1;
		"#,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await?;

	Ok(rows)
}

pub async fn get_deployment_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<Option<Deployment>, sqlx::Error> {
	let row = query_as!(
		Deployment,
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
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next();

	Ok(row)
}

pub async fn delete_deployment_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
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
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn generate_new_container_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	let mut uuid = Uuid::new_v4();

	let mut exists = query!(
		r#"
		SELECT
			*
		FROM
			deployment_runner
		WHERE
			container_id = $1;
		"#,
		uuid.as_bytes().as_ref()
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.is_some();

	while exists {
		uuid = Uuid::new_v4();
		exists = query!(
			r#"
			SELECT
				*
			FROM
				"user"
			WHERE
				id = $1;
			"#,
			uuid.as_bytes().as_ref()
		)
		.fetch_all(&mut *connection)
		.await?
		.into_iter()
		.next()
		.is_some();
	}

	Ok(uuid)
}

pub async fn get_inoperative_deployment_runner(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Option<DeploymentRunner>, sqlx::Error> {
	let row = query!(
		r#"
		SELECT
			*
		FROM
			deployment_runner
		WHERE
			container_id = NULL OR
			last_updated < $1;
		"#,
		(get_current_time_millis() - (1000 * 10)) as i64 // 10 seconds ago
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| DeploymentRunner {
		id: row.id,
		last_updated: row.last_updated as u64,
		container_id: row.container_id,
	});

	Ok(row)
}

pub async fn register_new_deployment_runner(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_runner
		VALUES
			($1, $2, $3);
		"#,
		runner_id,
		get_current_time_millis() as i64,
		runner_id,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_deployment_runner_container_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &[u8],
	container_id: Option<&[u8]>,
	old_container_id: Option<&[u8]>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment_runner
		SET
			container_id = $1
		WHERE
			id = $2 AND
			container_id = $3;
		"#,
		container_id,
		runner_id,
		old_container_id,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_deployment_runner_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &[u8],
) -> Result<Option<DeploymentRunner>, sqlx::Error> {
	let row = query!(
		r#"
		SELECT
			*
		FROM
			deployment_runner
		WHERE
			id = $1;
		"#,
		runner_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next()
	.map(|row| DeploymentRunner {
		id: row.id,
		last_updated: row.last_updated as u64,
		container_id: row.container_id,
	});

	Ok(row)
}

pub async fn register_deployment_application_server(
	connection: &mut <Database as sqlx::Database>::Connection,
	server_ip: &IpAddr,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			deployment_application_server
		VALUES
			($1, DEFAULT)
		ON CONFLICT DO NOTHING;
		"#,
		IpNetwork::from(server_ip.clone())
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_excess_deployment_application_servers(
	connection: &mut <Database as sqlx::Database>::Connection,
	servers: &[IpNetwork],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_application_server
		WHERE
			server_ip != ANY($1);
		"#,
		servers
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
