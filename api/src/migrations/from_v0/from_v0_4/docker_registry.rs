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

	add_docker_registry_info_permission(&mut *connection, config).await?;
	reset_permission_order(&mut *connection, config).await?;

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

async fn reset_permission_order(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	for permission in [
		// Domain permissions
		"workspace::domain::list",
		"workspace::domain::add",
		"workspace::domain::viewDetails",
		"workspace::domain::verify",
		"workspace::domain::delete",
		// Deployment permissions
		"workspace::deployment::list",
		"workspace::deployment::create",
		"workspace::deployment::info",
		"workspace::deployment::delete",
		"workspace::deployment::edit",
		// Upgrade path permissions
		"workspace::deployment::upgradePath::list",
		"workspace::deployment::upgradePath::create",
		"workspace::deployment::upgradePath::info",
		"workspace::deployment::upgradePath::delete",
		"workspace::deployment::upgradePath::edit",
		// Entry point permissions
		"workspace::deployment::entryPoint::list",
		"workspace::deployment::entryPoint::create",
		"workspace::deployment::entryPoint::edit",
		"workspace::deployment::entryPoint::delete",
		// Docker registry permissions
		"workspace::dockerRegistry::create",
		"workspace::dockerRegistry::list",
		"workspace::dockerRegistry::delete",
		"workspace::dockerRegistry::info",
		"workspace::dockerRegistry::push",
		"workspace::dockerRegistry::pull",
		// Managed database permissions
		"workspace::managedDatabase::create",
		"workspace::managedDatabase::list",
		"workspace::managedDatabase::delete",
		"workspace::managedDatabase::info",
		// Static site permissions
		"workspace::staticSite::list",
		"workspace::staticSite::create",
		"workspace::staticSite::info",
		"workspace::staticSite::delete",
		"workspace::staticSite::edit",
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
