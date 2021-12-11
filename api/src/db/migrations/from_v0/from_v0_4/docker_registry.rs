use crate::{migrate_query as query, Database};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
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
