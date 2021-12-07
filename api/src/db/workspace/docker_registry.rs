use api_models::models::workspace::docker_registry::{
	DockerRepositoryImageInfo,
	DockerRepositoryTagInfo,
};

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

	query!(
		r#"
		CREATE TABLE docker_registry_repository_digest(
			repository_id BYTEA NOT NULL
				CONSTRAINT docker_registry_repository_digest_fk_repository_id
					REFERENCES docker_registry_repository(id),
			digest TEXT NOT NULL,
			size BIGINT NOT NULL
				CONSTRAINT docker_registry_digest_chk_size_unsigned
					CHECK(size >= 0),
			created BIGINT NOT NULL CONSTRAINT
				docker_registry_digest_chk_created_unsigned CHECK(
					created >= 0
				),
			CONSTRAINT docker_registry_digest_pk PRIMARY KEY(
				repository_id, digest
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE docker_registry_digest_tag(
			repository_id BYTEA NOT NULL
				CONSTRAINT docker_registry_digest_tag_fk_repository_id
					REFERENCES docker_registry_repository(id),
			tag TEXT NOT NULL,
			digest TEXT NOT NULL,
			last_updated BIGINT NOT NULL CONSTRAINT
				docker_registry_digest_tag_chk_last_updated_unsigned CHECK(
					last_updated >= 0
				),
			CONSTRAINT docker_registry_digest_tag_pk PRIMARY KEY(
				repository_id, tag
			),
			CONSTRAINT docker_registry_digest_tag_fk_repository_id_digest
				FOREIGN KEY(repository_id, digest) REFERENCES
					docker_registry_repository_digest(repository_id, digest)
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
			name as "name: _"
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
			name as "name: _"
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
			name as "name: _"
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

pub async fn create_docker_repository_digest(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &[u8],
	digest: &str,
	size: u64,
	created: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			docker_registry_repository_digest
		VALUES
			($1, $2, $3, $4);
		"#,
		repository_id,
		digest,
		size as i64,
		created as i64
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn set_docker_repository_tag_details(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &[u8],
	tag: &str,
	digest: &str,
	last_updated: u64,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			docker_registry_digest_tag
		VALUES
			($1, $2, $3, $4)
		ON CONFLICT (repository_id, tag)
		DO UPDATE SET
			digest = $3,
			last_updated = $4;
		"#,
		repository_id,
		tag,
		digest,
		last_updated as i64
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_list_of_tags_for_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &[u8],
) -> Result<Vec<DockerRepositoryTagInfo>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			tag,
			last_updated
		FROM
			docker_registry_digest_tag
		WHERE
			repository_id = $1;
		"#,
		repository_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| DockerRepositoryTagInfo {
		tag: row.tag,
		last_updated: row.last_updated as u64,
	})
	.collect();

	Ok(rows)
}

pub async fn get_list_of_digests_for_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &[u8],
) -> Result<Vec<DockerRepositoryImageInfo>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			digest,
			size,
			created
		FROM
			docker_registry_repository_digest
		WHERE
			repository_id = $1;
		"#,
		repository_id
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| DockerRepositoryImageInfo {
		digest: row.digest,
		size: row.size as u64,
		created: row.created as u64,
	})
	.collect();

	Ok(rows)
}

pub async fn delete_docker_repository(
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

pub async fn delete_docker_repository_image(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &[u8],
	digest: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			docker_registry_repository_digest
		WHERE
			repository_id = $1 AND
			digest = $2;
		"#,
		repository_id,
		digest
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_tag_from_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &[u8],
	tag: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			docker_registry_digest_tag
		WHERE
			repository_id = $1 AND
			tag = $2;
		"#,
		repository_id,
		tag
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
