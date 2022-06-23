use api_models::utils::Uuid;

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	rbac_related_migrations(&mut *connection, config).await?;

	Ok(())
}

async fn rbac_related_migrations(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	log::info!("Migrating RBAC v2 related tables...");

	// block permission changes
	rename_role_permissions_resource_tables(&mut *connection, config).await?;
	create_role_block_permissions_resource_table(&mut *connection, config)
		.await?;

	// project related changes
	add_resource_type_and_permissions_for_projects(&mut *connection, config)
		.await?;
	reset_permission_order(&mut *connection, config).await?;
	reset_resource_types_order(&mut *connection, config).await?;

	create_project_tables(&mut *connection, config).await?;

	log::info!("Migrated RBAC v2 related tables.");
	Ok(())
}

async fn rename_role_permissions_resource_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE
			role_permissions_resource
		RENAME TO
			role_allow_permissions_resource;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_permissions_resource_type
		RENAME TO
			role_allow_permissions_resource_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_fk_role_id
		TO
			role_allow_permissions_resource_type_fk_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_fk_permission_id
		TO
			role_allow_permissions_resource_type_fk_permission_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_fk_resource_type_id
		TO
			role_allow_permissions_resource_type_fk_resource_type_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_pk
		TO
			role_allow_permissions_resource_type_pk;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_fk_role_id
		TO
			role_allow_permissions_resource_fk_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_fk_permission_id
		TO
			role_allow_permissions_resource_fk_permission_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_fk_resource_id
		TO
			role_allow_permissions_resource_fk_resource_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_pk
		TO
			role_allow_permissions_resource_pk;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_type_idx_role_id
		RENAME TO
			role_allow_permissions_resource_type_idx_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_type_idx_role_id_resource_type_id
		RENAME TO
			role_allow_permissions_resource_type_idx_roleid_resourcetypeid;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_idx_role_id
		RENAME TO
			role_allow_permissions_resource_idx_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_idx_role_id_resource_id
		RENAME TO
			role_allow_permissions_resource_idx_role_id_resource_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn create_role_block_permissions_resource_table(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE role_block_permissions_resource(
			role_id UUID
				CONSTRAINT role_block_permissions_resource_fk_role_id
					REFERENCES role(id),
			permission_id UUID
				CONSTRAINT role_block_permissions_resource_fk_permission_id
					REFERENCES permission(id),
			resource_id UUID
				CONSTRAINT role_block_permissions_resource_fk_resource_id
					REFERENCES resource(id),
			CONSTRAINT role_block_permissions_resource_pk
				PRIMARY KEY(role_id, permission_id, resource_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_block_permissions_resource_idx_role_id
		ON
			role_block_permissions_resource(role_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_block_permissions_resource_idx_role_id_resource_id
		ON
			role_block_permissions_resource(role_id, resource_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_resource_type_and_permissions_for_projects(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// add project as a resource type
	const PROJECT_RESOURCE_TYPE_NAME: &str = "project";
	let project_resource_type_id = loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
				SELECT
					*
				FROM
					resource_type
				WHERE
					id = $1;
				"#,
			&uuid
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			// That particular resource ID doesn't exist. Use it
			break uuid;
		}
	};

	query!(
		r#"
		INSERT INTO
			resource_type
		VALUES
			($1, $2, '');
		"#,
		&project_resource_type_id,
		PROJECT_RESOURCE_TYPE_NAME,
	)
	.execute(&mut *connection)
	.await?;

	// add permissions for project resource type
	for &permission in [
		"workspace::project::list",
		"workspace::project::create",
		"workspace::project::info",
		"workspace::project::delete",
		"workspace::project::edit",
	]
	.iter()
	{
		let uuid = loop {
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
				// That particular resource ID doesn't exist. Use it
				break uuid;
			}
		};
		query!(
			r#"
			INSERT INTO
				permission
			VALUES
				($1, $2, $3);
			"#,
			&uuid,
			permission,
			"",
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn reset_permission_order(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for permission in [
		// domain permissions
		"workspace::domain::list",
		"workspace::domain::add",
		"workspace::domain::viewDetails",
		"workspace::domain::verify",
		"workspace::domain::delete",
		// dns permissions
		"workspace::domain::dnsRecord::list",
		"workspace::domain::dnsRecord::add",
		"workspace::domain::dnsRecord::edit",
		"workspace::domain::dnsRecord::delete",
		// deployment permissions
		"workspace::infrastructure::deployment::list",
		"workspace::infrastructure::deployment::create",
		"workspace::infrastructure::deployment::info",
		"workspace::infrastructure::deployment::delete",
		"workspace::infrastructure::deployment::edit",
		// upgradePath permissions
		"workspace::infrastructure::upgradePath::list",
		"workspace::infrastructure::upgradePath::create",
		"workspace::infrastructure::upgradePath::info",
		"workspace::infrastructure::upgradePath::delete",
		"workspace::infrastructure::upgradePath::edit",
		// managedUrl permissions
		"workspace::infrastructure::managedUrl::list",
		"workspace::infrastructure::managedUrl::create",
		"workspace::infrastructure::managedUrl::edit",
		"workspace::infrastructure::managedUrl::delete",
		// managedDatabase permissions
		"workspace::infrastructure::managedDatabase::create",
		"workspace::infrastructure::managedDatabase::list",
		"workspace::infrastructure::managedDatabase::delete",
		"workspace::infrastructure::managedDatabase::info",
		// staticSite permissions
		"workspace::infrastructure::staticSite::list",
		"workspace::infrastructure::staticSite::create",
		"workspace::infrastructure::staticSite::info",
		"workspace::infrastructure::staticSite::delete",
		"workspace::infrastructure::staticSite::edit",
		// dockerRegistry permissions
		"workspace::dockerRegistry::create",
		"workspace::dockerRegistry::list",
		"workspace::dockerRegistry::delete",
		"workspace::dockerRegistry::info",
		"workspace::dockerRegistry::push",
		"workspace::dockerRegistry::pull",
		// secret permissions
		"workspace::secret::list",
		"workspace::secret::create",
		"workspace::secret::edit",
		"workspace::secret::delete",
		// role permissions
		"workspace::rbac::role::list",
		"workspace::rbac::role::create",
		"workspace::rbac::role::edit",
		"workspace::rbac::role::delete",
		// user permissions
		"workspace::rbac::user::list",
		"workspace::rbac::user::add",
		"workspace::rbac::user::remove",
		"workspace::rbac::user::updateRoles",
		// github permissions
		"workspace::ci::github::connect",
		"workspace::ci::github::activate",
		"workspace::ci::github::deactivate",
		"workspace::ci::github::viewBuilds",
		"workspace::ci::github::restartBuilds",
		"workspace::ci::github::disconnect",
		// project permissions
		"workspace::project::list",
		"workspace::project::create",
		"workspace::project::info",
		"workspace::project::delete",
		"workspace::project::edit",
		// workspace permissions
		"workspace::edit",
		"workspace::delete",
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

async fn reset_resource_types_order(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for resource_type in [
		"workspace",
		"domain",
		"dnsRecord",
		"dockerRepository",
		"managedDatabase",
		"deployment",
		"staticSite",
		"deploymentUpgradePath",
		"managedUrl",
		"secret",
		"project",
	] {
		query!(
			r#"
			UPDATE
				resource_type
			SET
				name = CONCAT('test::', name)
			WHERE
				name = $1;
			"#,
			&resource_type,
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			UPDATE
				resource_type
			SET
				name = $1
			WHERE
				name = CONCAT('test::', $1);
			"#,
			&resource_type,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn create_project_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE project (
			id UUID
				CONSTRAINT project_pk PRIMARY KEY,
			workspace_id UUID NOT NULL
				CONSTRAINT project_fk_workspace_id REFERENCES workspace(id),
			name CITEXT NOT NULL,
			description VARCHAR(500) NOT NULL,
			deleted BOOLEAN NOT NULL DEFAULT FALSE,

			CONSTRAINT project_fk_id_workspace_id
				FOREIGN KEY (id, workspace_id)
					REFERENCES resource(id, owner_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			project_uq_idx_name_workspace_id
		ON
			project(name, workspace_id)
		WHERE
			deleted IS FALSE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	// create resource_group table
	query!(
		r#"
		CREATE TABLE resource_group(
			id UUID
				CONSTRAINT resource_group_pk PRIMARY KEY,
			workspace_id UUID
				CONSTRAINT resource_group_fk_workspace_id REFERENCES workspace(id),
			project_id UUID
				CONSTRAINT resource_group_fk_project_id REFERENCES project(id),

			CONSTRAINT resource_group_chk_id CHECK (
				(
					workspace_id IS NOT NULL
					AND project_id IS NULL
					AND id = workspace_id
				)
				OR (
					workspace_id IS NULL
					AND project_id IS NOT NULL
					AND id = workspace_id
				)
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
