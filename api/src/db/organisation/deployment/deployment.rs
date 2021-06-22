use crate::{models::db_mapping::Deployment, query, query_as, Database};

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

	Ok(())
}

pub async fn initialize_deployment_post(
	transaction: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up deployment tables initialization");
	query!(
		r#"
		ALTER TABLE deployment ADD CONSTRAINT deployment_fk_id
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

pub async fn get_all_deployments(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Vec<Deployment>, sqlx::Error> {
	let rows = query_as!(
		Deployment,
		r#"
		SELECT
			*
		FROM
			deployment;
		"#
	)
	.fetch_all(&mut *connection)
	.await?;

	Ok(rows)
}
