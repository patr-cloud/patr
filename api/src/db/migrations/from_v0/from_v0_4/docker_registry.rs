use api_models::utils::Uuid;

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

	add_docker_registry_info_permission(&mut *connection).await?;
	reset_permission_order(&mut *connection).await?;

	Ok(())
}

async fn add_docker_registry_info_permission(
	connection: &mut <Database as sqlx::Database>::Connection,
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

async fn reset_permission_order(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	for permission in [
		// Domain permissions
		"workspace::domain::list",
		"workspace::domain::add",
		"workspace::domain::viewDetails",
		"workspace::domain::verify",
		"workspace::domain::delete",
		// Deployment permissions
		"workspace::infrastructure::deployment::list",
		"workspace::infrastructure::deployment::create",
		"workspace::infrastructure::deployment::info",
		"workspace::infrastructure::deployment::delete",
		"workspace::infrastructure::deployment::edit",
		// Upgrade path permissions
		"workspace::infrastructure::upgradePath::list",
		"workspace::infrastructure::upgradePath::create",
		"workspace::infrastructure::upgradePath::info",
		"workspace::infrastructure::upgradePath::delete",
		"workspace::infrastructure::upgradePath::edit",
		// Entry point permissions
		"workspace::infrastructure::entryPoint::list",
		"workspace::infrastructure::entryPoint::create",
		"workspace::infrastructure::entryPoint::edit",
		"workspace::infrastructure::entryPoint::delete",
		// Managed database permissions
		"workspace::infrastructure::managedDatabase::create",
		"workspace::infrastructure::managedDatabase::list",
		"workspace::infrastructure::managedDatabase::delete",
		"workspace::infrastructure::managedDatabase::info",
		// Static site permissions
		"workspace::infrastructure::staticSite::list",
		"workspace::infrastructure::staticSite::create",
		"workspace::infrastructure::staticSite::info",
		"workspace::infrastructure::staticSite::delete",
		"workspace::infrastructure::staticSite::edit",
		// Docker registry permissions
		"workspace::dockerRegistry::create",
		"workspace::dockerRegistry::list",
		"workspace::dockerRegistry::delete",
		"workspace::dockerRegistry::info",
		"workspace::dockerRegistry::push",
		"workspace::dockerRegistry::pull",
		// Workspace permissions
		"workspace::viewRoles",
		"workspace::createRole",
		"workspace::editRole",
		"workspace::deleteRole",
		"workspace::editInfo",
	] {
		query!(
			r#"
			UPDATE
				permission
			SET
				name = CONCAT('test::', name)
			WHERE
				name = $1;
			"#,
			permission,
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			UPDATE
				permission
			SET
				name = $1
			WHERE
				name = CONCAT('test::', $1);
			"#,
			&permission,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}
