use crate::{models::db_mapping::Deployment, query, query_as, Database};

pub async fn initialize_deployment_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE deployment(
			id BYTEA CONSTRAINT deployment_pk PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			registry VARCHAR(255) NOT NULL DEFAULT 'registry.vicara.tech',
			repository_id BYTEA CONSTRAINT deployment_fk_repository_id
				REFERENCES docker_registry_repository(id),
			image_name VARCHAR(512),
			image_tag VARCHAR(255) NOT NULL,
			deployed_image TEXT,
			deployment_id TEXT
				CONSTRAINT deployment_uq_deployment_id UNIQUE,
			CONSTRAINT deployment_chk_repository_id_is_valid CHECK(
				(
					registry = 'registry.vicara.tech' AND
					image_name IS NULL AND
					repository_id IS NOT NULL
				)
				OR
				(
					registry != 'registry.vicara.tech' AND
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

	// TODO this doesn't work as of now. Need to figure out GiST
	// query!(
	// 	r#"
	// 	CREATE INDEX
	// 		deployment_runner_idx_last_updated
	// 	ON
	// 		deployment_runner
	// 	USING GIST(last_updated);
	// 	"#
	// )
	// .execute(&mut *connection)
	// .await?;

	// query!(
	// 	r#"
	// 	CREATE INDEX
	// 		deployment_runner_deployment_idx_last_updated
	// 	ON
	// 		deployment_runner_deployment
	// 	USING GIST(last_updated);
	// 	"#
	// )
	// .execute(&mut *connection)
	// .await?;

	// query!(
	// 	r#"
	// 	CREATE INDEX
	// 		deployment_running_stats_idx_timestamp
	// 	ON
	// 		deployment_running_stats
	// 	USING GIST(timestamp);
	// 	"#
	// )
	// .execute(&mut *connection)
	// .await?;

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
			($1, $2, 'registry.vicara.tech', $3, NULL, $4, NULL, NULL);
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
			($1, $2, $3, NULL, $4, $5, NULL, NULL);
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
		LEFT JOIN
			docker_registry_repository
		ON
			docker_registry_repository.id = deployment.repository_id
		WHERE
			(
				(
					registry = 'registry.vicara.tech' AND
					docker_registry_repository.name = $1
				) OR
				(
					registry != 'registry.vicara.tech' AND
					image_name = $1
				)
			) AND
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

pub async fn update_deployment_deployed_image(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	deployed_image: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			deployed_image = $1
		WHERE
			id = $2;
		"#,
		deployed_image,
		deployment_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_digital_ocean_app_id_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	app_deployment_id: &str,
	deployment_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			digital_ocean_app_id = $1
		WHERE
			id = $2;
		"#,
		app_deployment_id,
		deployment_id
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
