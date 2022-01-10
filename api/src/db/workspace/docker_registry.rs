use crate::{models::db_mapping::DockerRepository, query, query_as, Database};

pub async fn initialize_docker_registry_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing docker registry tables");
	query!(
		r#"
		CREATE TABLE docker_registry_repository(
			id BYTEA CONSTRAINT docker_registry_repository_pk PRIMARY KEY,
			workspace_id BYTEA NOT NULL
				CONSTRAINT docker_registry_repository_fk_workspace_id
					REFERENCES workspace(id),
			name CITEXT NOT NULL,
			CONSTRAINT docker_registry_repository_uq_workspace_id_name
				UNIQUE(workspace_id, name)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_docker_registry_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up docker registry tables initialization");
	query!(
		r#"
		ALTER TABLE docker_registry_repository
		ADD CONSTRAINT docker_registry_repository_fk_id_workspace_id
		FOREIGN KEY(id, workspace_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

// function to add new repositorys
pub async fn create_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	resource_id: &[u8],
	name: &str,
	workspace_id: &[u8],
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			docker_registry_repository
		VALUES
			($1, $2, $3);
		"#,
		resource_id,
		workspace_id,
		name as _
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

pub async fn get_repository_by_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_name: &str,
	workspace_id: &[u8],
) -> Result<Option<DockerRepository>, sqlx::Error> {
	query_as!(
		DockerRepository,
		r#"
		SELECT
			id,
			workspace_id,
			name::TEXT as "name!: _"
		FROM
			docker_registry_repository
		WHERE
			name = $1
		AND
			workspace_id = $2 AND
			name NOT LIKE 'patr-deleted:%';
		"#,
		repository_name as _,
		workspace_id
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_docker_repositories_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &[u8],
) -> Result<Vec<DockerRepository>, sqlx::Error> {
	query_as!(
		DockerRepository,
		r#"
		SELECT
			id,
			workspace_id,
			name::TEXT as "name!: _"
		FROM
			docker_registry_repository
		WHERE
			workspace_id = $1 AND
			name NOT LIKE 'patr-deleted:%';
		"#,
		workspace_id
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_docker_repository_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &[u8],
) -> Result<Option<DockerRepository>, sqlx::Error> {
	query_as!(
		DockerRepository,
		r#"
		SELECT
			id,
			workspace_id,
			name::TEXT as "name!: _"
		FROM
			docker_registry_repository
		WHERE
			id = $1 AND
			name NOT LIKE 'patr-deleted:%';
		"#,
		repository_id
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_docker_repository_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &[u8],
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			docker_registry_repository
		SET
			name = $2
		WHERE
			id = $1;
		"#,
		repository_id,
		name as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
