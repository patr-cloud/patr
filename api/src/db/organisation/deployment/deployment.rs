use sqlx::{MySql, Transaction};

use crate::{
	models::db_mapping::{Deployment, VolumeMount},
	query,
	query_as,
};

pub async fn initialize_deployment_pre(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment (
			id BINARY(16) PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			registry VARCHAR(255) NOT NULL DEFAULT "registry.docker.vicara.co",
			repository_id BINARY(16) NULL,
			image_name VARCHAR(512) NULL,
			image_tag VARCHAR(255) NOT NULL,
			FOREIGN KEY (repository_id) REFERENCES docker_registry_repository(id),
			CONSTRAINT CHECK 
			(			
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
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment_exposed_port(
			deployment_id BINARY(16),
			port SMALLINT UNSIGNED NOT NULL,
			PRIMARY KEY (deployment_id, port),
			FOREIGN KEY (deployment_id) REFERENCES deployment(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment_environment_variable(
			deployment_id BINARY(16),
			name VARCHAR(50) NOT NULL,
			value VARCHAR(50) NOT NULL,
			PRIMARY KEY (deployment_id, name),
			FOREIGN KEY (deployment_id) REFERENCES deployment(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment_persistent_volume(
			deployment_id BINARY(16),
			name VARCHAR(50) NOT NULL,
			path VARCHAR(255) NOT NULL,
			PRIMARY KEY (deployment_id, name),
			UNIQUE (deployment_id, path),
			FOREIGN KEY (deployment_id) REFERENCES deployment(id)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_deployment_post(
	transaction: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up deployment tables initialization");
	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT
		FOREIGN KEY (id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn create_deployment_with_internal_registry(
	connection: &mut Transaction<'_, MySql>,
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
			(?, ?, "registry.docker.vicara.co", ?, NULL, ?);
		"#,
		deployment_id,
		name,
		repository_id,
		image_tag
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn create_deployment_with_external_registry(
	connection: &mut Transaction<'_, MySql>,
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
			(?, ?, ?, NULL, ?, ?);
		"#,
		deployment_id,
		name,
		registry,
		image_name,
		image_tag
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn get_deployments_by_image_name_and_tag_for_organisation(
	connection: &mut Transaction<'_, MySql>,
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
	connection: &mut Transaction<'_, MySql>,
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
	connection: &mut Transaction<'_, MySql>,
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

pub async fn delete_deployment_by_id(
	connection: &mut Transaction<'_, MySql>,
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

pub async fn get_exposed_ports_for_deployment(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
) -> Result<Vec<u16>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			deployment_exposed_port
		WHERE
			deployment_id = ?;
		"#,
		deployment_id
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.map(|row| row.port)
	.collect();

	Ok(rows)
}

pub async fn add_exposed_port_for_deployment(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
	port: u16,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			deployment_exposed_port
		VALUES
			(?, ?);
		"#,
		deployment_id,
		port
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_exposed_ports_for_deployment(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_exposed_port
		WHERE
			deployment_id = ?;
		"#,
		deployment_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn get_persistent_volumes_for_deployment(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
) -> Result<Vec<VolumeMount>, sqlx::Error> {
	query_as!(
		VolumeMount,
		r#"
		SELECT
			*
		FROM
			deployment_persistent_volume
		WHERE
			deployment_id = ?;
		"#,
		deployment_id
	)
	.fetch_all(connection)
	.await
}

pub async fn add_persistent_volume_for_deployment(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
	name: &str,
	path: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			deployment_persistent_volume
		VALUES
			(?, ?, ?);
		"#,
		deployment_id,
		name,
		path
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_persistent_volumes_for_deployment(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_persistent_volume
		WHERE
			deployment_id = ?;
		"#,
		deployment_id
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn get_environment_variables_for_deployment(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
) -> Result<Vec<(String, String)>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			deployment_environment_variable
		WHERE
			deployment_id = ?;
		"#,
		deployment_id
	)
	.fetch_all(connection)
	.await?
	.into_iter()
	.map(|row| (row.name, row.value))
	.collect();

	Ok(rows)
}

pub async fn add_environment_variable_for_deployment(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
	key: &str,
	value: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			deployment_environment_variable
		VALUES
			(?, ?, ?);
		"#,
		deployment_id,
		key,
		value
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_environment_variables_for_deployment(
	connection: &mut Transaction<'_, MySql>,
	deployment_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_environment_variable
		WHERE
			deployment_id = ?;
		"#,
		deployment_id,
	)
	.execute(connection)
	.await
	.map(|_| ())
}
