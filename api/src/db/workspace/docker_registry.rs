use api_models::{
	models::workspace::docker_registry::{
		DockerRepositoryImageInfo,
		DockerRepositoryTagInfo,
	},
	utils::Uuid,
};
use chrono::{DateTime, Utc};
use num_traits::ToPrimitive;

use crate::{query, query_as, Database};

pub struct DockerRepository {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub name: String,
}

pub async fn initialize_docker_registry_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing docker registry tables");
	query!(
		r#"
		CREATE TABLE docker_registry_repository(
			id UUID CONSTRAINT docker_registry_repository_pk PRIMARY KEY,
			workspace_id UUID NOT NULL
				CONSTRAINT docker_registry_repository_fk_workspace_id
					REFERENCES workspace(id),
			name CITEXT NOT NULL,
			CONSTRAINT docker_registry_repository_uq_workspace_id_name
				UNIQUE(workspace_id, name),
			CONSTRAINT docker_registry_repository_uq_id_workspace_id UNIQUE(
				id, workspace_id
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE docker_registry_repository_manifest(
			repository_id UUID NOT NULL
				CONSTRAINT docker_registry_repository_manifest_fk_repository_id
					REFERENCES docker_registry_repository(id),
			manifest_digest TEXT NOT NULL,
			size BIGINT NOT NULL
				CONSTRAINT
					docker_registry_repository_manifest_chk_size_unsigned
						CHECK(size >= 0),
			created TIMESTAMPTZ NOT NULL,
			CONSTRAINT docker_registry_repository_manifest_pk PRIMARY KEY(
				repository_id, manifest_digest
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE docker_registry_repository_tag(
			repository_id UUID NOT NULL
				CONSTRAINT docker_registry_repository_tag_fk_repository_id
					REFERENCES docker_registry_repository(id),
			tag TEXT NOT NULL,
			manifest_digest TEXT NOT NULL,
			last_updated TIMESTAMPTZ NOT NULL,
			CONSTRAINT docker_registry_repository_tag_pk PRIMARY KEY(
				repository_id, tag
			),
			CONSTRAINT
				docker_registry_repository_tag_fk_repository_id_manifest_digest
				FOREIGN KEY(repository_id, manifest_digest) REFERENCES
					docker_registry_repository_manifest(
						repository_id, manifest_digest
					)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// TODO indexes for size, last_updated, etc

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
	resource_id: &Uuid,
	name: &str,
	workspace_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			docker_registry_repository(
				id,
				workspace_id,
				name
			)
		VALUES
			($1, $2, $3);
		"#,
		resource_id as _,
		workspace_id as _,
		name as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_docker_repository_by_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_name: &str,
	workspace_id: &Uuid,
) -> Result<Option<DockerRepository>, sqlx::Error> {
	query_as!(
		DockerRepository,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
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
		workspace_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_docker_repositories_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<(DockerRepository, u64, DateTime<Utc>)>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			id as "id: Uuid",
			workspace_id as "workspace_id: Uuid",
			name::TEXT as "name!: String",
			COALESCE(size, 0) as "size!",
			(
				SELECT
					GREATEST(
						resource.created,
						(
							SELECT
								COALESCE(created, TO_TIMESTAMP(0))
							FROM
								docker_registry_repository_manifest
							WHERE
								repository_id = docker_registry_repository.id
							ORDER BY
								created DESC
							LIMIT 1
						),
						(
							SELECT
								COALESCE(last_updated, TO_TIMESTAMP(0))
							FROM
								docker_registry_repository_tag
							WHERE
								repository_id = docker_registry_repository.id
							ORDER BY
								created DESC
							LIMIT 1
						)
					)
				FROM
					resource
				WHERE
					resource.id = docker_registry_repository.id
			) as "last_updated!"
		FROM
			docker_registry_repository
		LEFT JOIN (
			SELECT
				SUM(size) as size,
				repository_id
			FROM
				docker_registry_repository_manifest
			GROUP BY
				repository_id
		) docker_registry_repository_manifest
		ON
			docker_registry_repository_manifest.repository_id =
				docker_registry_repository.id
		WHERE
			workspace_id = $1 AND
			name NOT LIKE 'patr-deleted:%';
		"#,
		workspace_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| {
		(
			DockerRepository {
				id: row.id,
				name: row.name,
				workspace_id: row.workspace_id,
			},
			row.size.to_u64().unwrap_or(0),
			row.last_updated,
		)
	})
	.collect();

	Ok(rows)
}

pub async fn get_docker_repository_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
) -> Result<Option<DockerRepository>, sqlx::Error> {
	query_as!(
		DockerRepository,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			name::TEXT as "name!: _"
		FROM
			docker_registry_repository
		WHERE
			id = $1 AND
			name NOT LIKE 'patr-deleted:%';
		"#,
		repository_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_docker_repository_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
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
		repository_id as _,
		name as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn create_docker_repository_digest(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
	digest: &str,
	size: u64,
	created: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			docker_registry_repository_manifest(
				repository_id,
				manifest_digest,
				size,
				created
			)
		VALUES
			($1, $2, $3, $4)
		ON CONFLICT DO NOTHING;
		"#,
		repository_id as _,
		digest,
		size as i64,
		created as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn set_docker_repository_tag_details(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
	tag: &str,
	digest: &str,
	last_updated: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			docker_registry_repository_tag(
				repository_id,
				tag,
				manifest_digest,
				last_updated
			)
		VALUES
			($1, $2, $3, $4)
		ON CONFLICT(repository_id, tag)
		DO UPDATE SET
			manifest_digest = $3,
			last_updated = $4;
		"#,
		repository_id as _,
		tag,
		digest,
		last_updated as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_list_of_tags_for_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
) -> Result<Vec<(DockerRepositoryTagInfo, String)>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			tag,
			manifest_digest,
			last_updated
		FROM
			docker_registry_repository_tag
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| {
		(
			DockerRepositoryTagInfo {
				tag: row.tag,
				last_updated: row.last_updated.into(),
			},
			row.manifest_digest,
		)
	})
	.collect();

	Ok(rows)
}

pub async fn get_tags_for_docker_repository_image(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
	digest: &str,
) -> Result<Vec<DockerRepositoryTagInfo>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			tag,
			last_updated
		FROM
			docker_registry_repository_tag
		WHERE
			repository_id = $1 AND
			manifest_digest = $2;
		"#,
		repository_id as _,
		digest
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| DockerRepositoryTagInfo {
		tag: row.tag,
		last_updated: row.last_updated.into(),
	})
	.collect();

	Ok(rows)
}

pub async fn get_total_size_of_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
) -> Result<u64, sqlx::Error> {
	query!(
		r#"
		SELECT
			COALESCE(SUM(size), 0)::BIGINT as "size!"
		FROM
			docker_registry_repository_manifest
		WHERE
			repository_id = $1;
		"#,
		repository_id as _,
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| row.size as u64)
}

pub async fn get_total_size_of_docker_repositories_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<u64, sqlx::Error> {
	query!(
		r#"
		SELECT
			COALESCE(SUM(size), 0)::BIGINT as "size!"
		FROM
			docker_registry_repository_manifest
		INNER JOIN
			docker_registry_repository
		ON
			docker_registry_repository.id = docker_registry_repository_manifest.repository_id
		WHERE
			docker_registry_repository.workspace_id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| row.size as u64)
}

pub async fn get_docker_repository_image_by_digest(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
	digest: &str,
) -> Result<Option<DockerRepositoryImageInfo>, sqlx::Error> {
	query!(
		r#"
		SELECT
			manifest_digest,
			size,
			created
		FROM
			docker_registry_repository_manifest
		WHERE
			repository_id = $1 AND
			manifest_digest = $2;
		"#,
		repository_id as _,
		digest
	)
	.fetch_optional(&mut *connection)
	.await
	.map(|row| {
		row.map(|row| DockerRepositoryImageInfo {
			digest: row.manifest_digest,
			size: row.size as u64,
			created: row.created.into(),
		})
	})
}

pub async fn get_docker_repository_tag_details(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
	tag: &str,
) -> Result<Option<(DockerRepositoryTagInfo, String)>, sqlx::Error> {
	query!(
		r#"
		SELECT
			tag,
			last_updated,
			manifest_digest
		FROM
			docker_registry_repository_tag
		WHERE
			repository_id = $1 AND
			tag = $2;
		"#,
		repository_id as _,
		tag
	)
	.fetch_optional(&mut *connection)
	.await
	.map(|row| {
		row.map(|row| {
			(
				DockerRepositoryTagInfo {
					tag: row.tag,
					last_updated: row.last_updated.into(),
				},
				row.manifest_digest,
			)
		})
	})
}

pub async fn get_list_of_digests_for_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
) -> Result<Vec<DockerRepositoryImageInfo>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			manifest_digest,
			size,
			created
		FROM
			docker_registry_repository_manifest
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| DockerRepositoryImageInfo {
		digest: row.manifest_digest,
		size: row.size as u64,
		created: row.created.into(),
	})
	.collect();

	Ok(rows)
}

pub async fn get_latest_digest_for_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
) -> Result<Option<String>, sqlx::Error> {
	let digest = query!(
		r#"
		SELECT
			manifest_digest
		FROM
			docker_registry_repository_manifest
		WHERE
			repository_id = $1
		ORDER BY
			created DESC
		LIMIT 1;
		"#,
		repository_id as _
	)
	.fetch_optional(&mut *connection)
	.await?
	.map(|row| row.manifest_digest);

	Ok(digest)
}

pub async fn delete_all_tags_for_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			docker_registry_repository_tag
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_all_images_for_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			docker_registry_repository_manifest
		WHERE
			repository_id = $1;
		"#,
		repository_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

#[allow(dead_code)]
pub async fn delete_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			docker_registry_repository
		WHERE
			id = $1;
		"#,
		repository_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_docker_repository_image(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
	digest: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			docker_registry_repository_manifest
		WHERE
			repository_id = $1 AND
			manifest_digest = $2;
		"#,
		repository_id as _,
		digest
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn delete_tag_from_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
	tag: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			docker_registry_repository_tag
		WHERE
			repository_id = $1 AND
			tag = $2;
		"#,
		repository_id as _,
		tag
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_last_updated_for_docker_repository(
	connection: &mut <Database as sqlx::Database>::Connection,
	repository_id: &Uuid,
) -> Result<DateTime<Utc>, sqlx::Error> {
	query!(
		r#"
		SELECT 
			GREATEST(
				resource.created, 
				(
					SELECT 
						COALESCE(created, TO_TIMESTAMP(0)) 
					FROM 
						docker_registry_repository_manifest 
					WHERE 
						repository_id = $1
					ORDER BY
						created DESC
					LIMIT 1
				), 
				(
					SELECT 
						COALESCE(last_updated, TO_TIMESTAMP(0)) 
					FROM 
						docker_registry_repository_tag 
					WHERE 
						repository_id = $1
					ORDER BY
						created DESC
					LIMIT 1
				)
			) as "last_updated!"
		FROM
			resource
		WHERE
			resource.id = $1;
		"#,
		repository_id as _
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| row.last_updated)
}
