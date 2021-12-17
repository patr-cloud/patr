use crate::{migrate_query as query, Database};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
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
			created BIGINT NOT NULL CONSTRAINT
				docker_registry_repository_manifest_chk_created_unsigned CHECK(
					created >= 0
				),
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
			last_updated BIGINT NOT NULL CONSTRAINT
				docker_registry_repository_tag_chk_last_updated_unsigned CHECK(
					last_updated >= 0
				),
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

	Ok(())
}
