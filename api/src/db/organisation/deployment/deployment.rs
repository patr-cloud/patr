use crate::{
	models::db_mapping::{Deployment, VolumeMount},
	query,
	query_as,
	Database,
};

pub async fn initialize_deployment_pre(
	transaction: &mut <Database as sqlx::Database>::Connection,
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
			upgrade_path_id BYTEA NOT NULL,
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
		CREATE TABLE deployment_exposed_port(
			deployment_id BYTEA CONSTRAINT deployment_exposed_fk_deployment_id
				REFERENCES deployment(id),
			port INTEGER NOT NULL
				CONSTRAINT deployment_exposed_port_chk_port_u16
					CHECK(port >= 0 AND port <= 65534),
			CONSTRAINT deployment_exposed_port_pk
				PRIMARY KEY(deployment_id, port)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_environment_variable(
			deployment_id BYTEA
				CONSTRAINT deployment_environment_variable_fk_deployment_id
					REFERENCES deployment(id),
			name VARCHAR(50) NOT NULL,
			value VARCHAR(50) NOT NULL,
			CONSTRAINT deployment_environment_variable_pk
				PRIMARY KEY(deployment_id, name)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_persistent_volume(
			deployment_id BYTEA
				CONSTRAINT deployment_persistent_volume_fk_deployment_id
					REFERENCES deployment(id),
			name VARCHAR(50) NOT NULL,
			path VARCHAR(255) NOT NULL,
			CONSTRAINT deployment_persistent_volume_pk
				PRIMARY KEY(deployment_id, name),
			CONSTRAINT deployment_persistent_volume_uq_deployment_id_path
				UNIQUE(deployment_id, path)
		);
		"#
	)
	.execute(&mut *transaction)
	.await?;

	Ok(())
}

pub async fn initialize_deployment_post(
	transaction: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up deployment tables initialization");
	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *transaction)
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
			($1, $2, 'registry.docker.vicara.co', $3, NULL, $4);
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
			($1, $2, $3, NULL, $4, $5);
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
	query_as!(
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
	.await
}

pub async fn get_deployments_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
	organisation_id: &[u8],
) -> Result<Vec<Deployment>, sqlx::Error> {
	query_as!(
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
	.await
}

pub async fn get_deployment_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
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
			id = $1;
		"#,
		deployment_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.next())
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

pub async fn get_exposed_ports_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<Vec<u16>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.port as u16)
	.collect();

	Ok(rows)
}

pub async fn add_exposed_port_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	port: u16,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			deployment_exposed_port
		VALUES
			($1, $2);
		"#,
		deployment_id,
		port as i32
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_exposed_ports_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_persistent_volumes_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
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
			deployment_id = $1;
		"#,
		deployment_id
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn add_persistent_volume_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	name: &str,
	path: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			deployment_persistent_volume
		VALUES
			($1, $2, $3);
		"#,
		deployment_id,
		name,
		path
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_persistent_volumes_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_persistent_volume
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_environment_variables_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<Vec<(String, String)>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			*
		FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.name, row.value))
	.collect();

	Ok(rows)
}

pub async fn add_environment_variable_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	key: &str,
	value: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			deployment_environment_variable
		VALUES
			($1, $2, $3);
		"#,
		deployment_id,
		key,
		value
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_environment_variables_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
