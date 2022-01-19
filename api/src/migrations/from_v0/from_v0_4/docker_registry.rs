use api_models::utils::Uuid;

use crate::{migrate_query as query, utils::settings::Settings, Database};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
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

	query!(
		r#"
		ALTER TABLE docker_registry_repository
		ADD CONSTRAINT docker_registry_repository_uq_id_workspace_id
		UNIQUE(id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	add_docker_registry_info_permission(&mut *connection, config).await?;

	Ok(())
}

async fn add_docker_registry_info_permission(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	let permission_id = loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				permission
			WHERE
				id = $1;
			"#,
			&uuid
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break uuid;
		}
	};

	query!(
		r#"
		INSERT INTO
			permission
		VALUES
			($1, $2, NULL);
		"#,
		permission_id,
		"workspace::dockerRegistry::info"
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
