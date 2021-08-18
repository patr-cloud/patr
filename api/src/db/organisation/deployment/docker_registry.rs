use crate::{models::db_mapping::DockerRepository, query, query_as, Database};

pub async fn initialize_docker_registry_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing docker registry tables");
	query!(
		r#"
		CREATE TABLE docker_registry_repository(
			id BYTEA CONSTRAINT docker_registry_repository_pk PRIMARY KEY,
			organisation_id BYTEA NOT NULL
				CONSTRAINT docker_registry_repository_fk_id
					REFERENCES organisation(id),
			name VARCHAR(255) NOT NULL,
			CONSTRAINT docker_registry_repository_uq_organisation_id_name
				UNIQUE(organisation_id, name)
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
		ADD CONSTRAINT docker_registry_repository_fk_id_organisation_id
		FOREIGN KEY(id, organisation_id) REFERENCES resource(id, owner_id);
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
	.execute(&mut *connection)
	.await?;
	Ok(())
}

pub async fn get_repository_by_name(
	connection: &mut <Database as sqlx::Database>::Connection,
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
			name = $1
		AND
			organisation_id = $2;
		"#,
		repository_name,
		organisation_id
	)
	.fetch_all(&mut *connection)
	.await
	.map(|rows| rows.into_iter().next())
}

pub async fn get_docker_repositories_for_organisation(
	connection: &mut <Database as sqlx::Database>::Connection,
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
			*
		FROM
			docker_registry_repository
		WHERE
			id = $1;
		"#,
		repository_id
	)
	.fetch_all(&mut *connection)
	.await
	.map(|repos| repos.into_iter().next())
}

pub async fn delete_docker_repository_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
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
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
