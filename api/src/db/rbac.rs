use models::rbac::{Permission, ResourceType};

use crate::prelude::*;

/// Initializes the rbac tables
#[instrument(skip(connection))]
pub async fn initialize_rbac_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up rbac tables");

	// Resource types, like application, deployment, VM, etc
	query!(
		r#"
		CREATE TABLE resource_type(
			id UUID NOT NULL,
			name VARCHAR(100) NOT NULL,
			description VARCHAR(500) NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE resource(
			id UUID NOT NULL,
			resource_type_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Roles belong to an workspace. Immutable roles are the defaults seeded
	// at workspace creation — uneditable and undeletable.
	query!(
		r#"
		CREATE TABLE role(
			id UUID NOT NULL,
			name VARCHAR(100) NOT NULL,
			description VARCHAR(500) NOT NULL,
			workspace_id UUID NOT NULL,
			is_immutable BOOLEAN NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// A role is a flat list of permissions; targeting lives on the binding.
	query!(
		r#"
		CREATE TABLE role_permission(
			role_id UUID NOT NULL,
			permission_id UUID NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE permission(
			id UUID NOT NULL,
			name VARCHAR(100) NOT NULL,
			description VARCHAR(500) NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// The workspace-scoped principal that role bindings are granted on. A
	// user's actor requires membership. One actor per principal per workspace.
	// Service accounts get their own variant and column when that table lands.
	query!(
		r#"
		CREATE TYPE WORKSPACE_ACTOR_TYPE AS ENUM(
			'user'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE workspace_actor(
			id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			actor_type WORKSPACE_ACTOR_TYPE NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Users belong to an workspace through a role
	query!(
		r#"
		CREATE TABLE workspace_user(
			user_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			role_id UUID NOT NULL,
			actor_id UUID,
			actor_type WORKSPACE_ACTOR_TYPE NOT NULL
				GENERATED ALWAYS AS ('user') STORED
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE role_resource_permissions_type(
			role_id UUID NOT NULL,
			permission_id UUID NOT NULL,
			permission_type PERMISSION_TYPE NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE role_resource_permissions_include(
			role_id UUID NOT NULL,
			permission_id UUID NOT NULL,
			resource_id UUID NOT NULL,
			permission_type PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('include') STORED
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE role_resource_permissions_exclude(
			role_id UUID NOT NULL,
			permission_id UUID NOT NULL,
			resource_id UUID NOT NULL,
			permission_type PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('exclude') STORED
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the rbac indices
#[instrument(skip(connection))]
pub async fn initialize_rbac_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up rbac table indices");

	// Resource types, like application, deployment, VM, etc
	query!(
		r#"
		ALTER TABLE resource_type
			ADD CONSTRAINT resource_type_pk PRIMARY KEY(id),
			ADD CONSTRAINT resource_type_uq_name UNIQUE(name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE resource
			ADD CONSTRAINT resource_pk PRIMARY KEY(id),
			ADD CONSTRAINT resource_uq_id_workspace_id UNIQUE(id, workspace_id),
			ADD CONSTRAINT resource_uq_id_workspace_id_deleted UNIQUE(id, workspace_id, deleted);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			resource_idx_workspace_id
		ON
			resource
		(workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Roles belong to an workspace
	query!(
		r#"
		ALTER TABLE role
			ADD CONSTRAINT role_pk
				PRIMARY KEY(id),
			ADD CONSTRAINT role_fk_id_workspace_id
				FOREIGN KEY(id, workspace_id) REFERENCES resource(id, workspace_id),
			ADD CONSTRAINT role_uq_name_workspace_id
				UNIQUE(name, workspace_id),
			ADD CONSTRAINT role_uq_id_workspace_id
				UNIQUE(id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE permission
			ADD CONSTRAINT permission_pk PRIMARY KEY(id),
			ADD CONSTRAINT permission_uq_name UNIQUE(name);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_permission
		ADD CONSTRAINT role_permission_pk
		PRIMARY KEY(role_id, permission_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_actor
			ADD CONSTRAINT workspace_actor_pk PRIMARY KEY(id),
			ADD CONSTRAINT workspace_actor_uq_id_workspace_id UNIQUE(id, workspace_id),
			ADD CONSTRAINT workspace_actor_uq_id_actor_type UNIQUE(id, actor_type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Users belong to an workspace through a role
	query!(
		r#"
		ALTER TABLE workspace_user
		ADD CONSTRAINT workspace_user_pk
		PRIMARY KEY(user_id, workspace_id, role_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			workspace_user_idx_user_id
		ON
			workspace_user
		(user_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			workspace_user_idx_user_id_workspace_id
		ON
			workspace_user
		(user_id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_resource_permissions_type
			ADD CONSTRAINT role_resource_permissions_type_pk PRIMARY KEY(
				role_id,
				permission_id
			),
			ADD CONSTRAINT role_resource_permissions_type_uq UNIQUE(
				role_id,
				permission_id,
				permission_type
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_resource_permissions_include
		ADD CONSTRAINT role_resource_permissions_include_pk
		PRIMARY KEY(role_id, permission_id, resource_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_resource_permissions_exclude
		ADD CONSTRAINT role_resource_permissions_exclude_pk
		PRIMARY KEY(role_id,permission_id,resource_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the rbac constraints
#[instrument(skip(connection))]
pub async fn initialize_rbac_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up rbac table constraints");

	query!(
		r#"
		ALTER TABLE resource
			ADD CONSTRAINT resource_fk_resource_type_id
				FOREIGN KEY(resource_type_id) REFERENCES resource_type(id),
			ADD CONSTRAINT resource_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id)
					DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Roles belong to an workspace
	query!(
		r#"
		ALTER TABLE role
		ADD CONSTRAINT role_fk_workspace_id
		FOREIGN KEY(workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_permission
			ADD CONSTRAINT role_permission_fk_role_id
				FOREIGN KEY(role_id) REFERENCES role(id),
			ADD CONSTRAINT role_permission_fk_permission_id
				FOREIGN KEY(permission_id) REFERENCES permission(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// The (user_id, workspace_id) FK up to workspace_user arrives with the
	// cutover, once that table's primary key no longer carries role_id.
	query!(
		r#"
		ALTER TABLE workspace_actor
			ADD CONSTRAINT workspace_actor_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Users belong to an workspace through a role
	query!(
		r#"
		ALTER TABLE workspace_user
			ADD CONSTRAINT workspace_user_fk_user_id
				FOREIGN KEY(user_id) REFERENCES "user"(id),
			ADD CONSTRAINT workspace_user_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id),
			ADD CONSTRAINT workspace_user_fk_role_id_workspace_id
				FOREIGN KEY(role_id, workspace_id) REFERENCES role(id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_resource_permissions_type
			ADD CONSTRAINT role_resource_permissions_type_fk_role_id
				FOREIGN KEY(role_id) REFERENCES role(id),
			ADD CONSTRAINT role_resource_permissions_type_fk_permission_id
				FOREIGN KEY(permission_id) REFERENCES permission(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_resource_permissions_include
			ADD CONSTRAINT role_resource_permissions_include_fk_parent FOREIGN KEY(
				role_id,
				permission_id,
				permission_type
			) REFERENCES role_resource_permissions_type(
				role_id,
				permission_id,
				permission_type
			),
			ADD CONSTRAINT role_resource_permissions_include_fk_resource
				FOREIGN KEY(resource_id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_resource_permissions_exclude
			ADD CONSTRAINT role_resource_permissions_exclude_fk_parent
				FOREIGN KEY(role_id, permission_id, permission_type)
					REFERENCES role_resource_permissions_type(
						role_id,
						permission_id,
						permission_type
					),
			ADD CONSTRAINT role_resource_permissions_exclude_fk_resource
				FOREIGN KEY(resource_id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Insert all permissions into the database
	for permission in Permission::list_all() {
		trace!("Inserting permission: {}", permission);
		query!(
			r#"
			INSERT INTO
				permission(
					id,
					name,
					description
				)
			VALUES
				($1, $2, $3);
			"#,
			Uuid::new_v4() as _,
			permission.to_string(),
			permission.description()
		)
		.execute(&mut *connection)
		.await?;
	}

	// Insert all resource types into the database
	for resource_type in ResourceType::list_all() {
		trace!("Inserting resource type: {}", resource_type);
		query!(
			r#"
			INSERT INTO
				resource_type(
					id,
					name,
					description
				)
			VALUES
				($1, $2, $3);
			"#,
			Uuid::new_v4() as _,
			resource_type.to_string(),
			resource_type.description()
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		CREATE FUNCTION GENERATE_RESOURCE_ID() RETURNS UUID AS $$
		DECLARE
			resource_id UUID;
		BEGIN
			resource_id := gen_random_uuid();
			WHILE EXISTS(
				SELECT
					1
				FROM
					resource
				WHERE
					id = resource_id
			) LOOP
				resource_id := gen_random_uuid();
			END LOOP;
			RETURN resource_id;
		END;
		$$ LANGUAGE plpgsql;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE FUNCTION RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID(
			login_id UUID,
			permission_name TEXT
		) RETURNS TABLE(
			id UUID,
			resource_type_id UUID,
			workspace_id UUID,
			created TIMESTAMPTZ,
			deleted TIMESTAMPTZ
		) AS $$
		DECLARE
			local_permission_id UUID;
		BEGIN
			/* Resolve permission name to ID */
			SELECT
				permission.id
			INTO
				local_permission_id
			FROM
				permission
			WHERE
				name = permission_name;

			IF local_permission_id IS NULL THEN
				RAISE EXCEPTION 'Permission `%` not found', permission_name;
			END IF;

			RETURN QUERY
			/* Workspaces where this login has super admin access */
			WITH super_admin_workspaces AS (
				SELECT
					workspace.id AS workspace_id
				FROM
					web_login
				INNER JOIN
					workspace
				ON
					workspace.super_admin_id = web_login.user_id
				WHERE
					web_login.login_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
				UNION ALL
				SELECT
					user_api_token_workspace_super_admin.workspace_id
				FROM
					user_api_token_workspace_super_admin
				WHERE
					user_api_token_workspace_super_admin.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
			),
			/* Resources explicitly granted via include lists,
			scoped to the workspace the role belongs to */
			included_resources AS (
				SELECT
					role_resource_permissions_include.resource_id,
					workspace_user.workspace_id
				FROM
					web_login
				INNER JOIN
					workspace_user
				ON
					workspace_user.user_id = web_login.user_id
				INNER JOIN
					role_resource_permissions_include
				ON
					role_resource_permissions_include.role_id = workspace_user.role_id AND
					role_resource_permissions_include.permission_id = local_permission_id
				WHERE
					web_login.login_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
				UNION ALL
				SELECT
					user_api_token_resource_permissions_include.resource_id,
					user_api_token_resource_permissions_include.workspace_id
				FROM
					user_api_token_resource_permissions_include
				WHERE
					user_api_token_resource_permissions_include.permission_id = local_permission_id AND
					user_api_token_resource_permissions_include.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
			),
			/* Resources explicitly denied via exclude lists */
			excluded_resources AS (
				SELECT
					role_resource_permissions_exclude.resource_id
				FROM
					web_login
				INNER JOIN
					workspace_user
				ON
					workspace_user.user_id = web_login.user_id
				INNER JOIN
					role_resource_permissions_exclude
				ON
					role_resource_permissions_exclude.role_id = workspace_user.role_id AND
					role_resource_permissions_exclude.permission_id = local_permission_id
				WHERE
					web_login.login_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
				UNION ALL
				SELECT
					user_api_token_resource_permissions_exclude.resource_id
				FROM
					user_api_token_resource_permissions_exclude
				WHERE
					user_api_token_resource_permissions_exclude.permission_id = local_permission_id AND
					user_api_token_resource_permissions_exclude.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
			),
			/* Workspaces where this login has any exclude-type permission */
			exclude_workspaces AS (
				SELECT
					workspace_user.workspace_id
				FROM
					web_login
				INNER JOIN
					workspace_user
				ON
					workspace_user.user_id = web_login.user_id
				INNER JOIN
					role_resource_permissions_type
				ON
					role_resource_permissions_type.role_id = workspace_user.role_id AND
					role_resource_permissions_type.permission_id = local_permission_id AND
					role_resource_permissions_type.permission_type = 'exclude'
				WHERE
					web_login.login_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
				UNION ALL
				SELECT
					user_api_token_resource_permissions_type.workspace_id
				FROM
					user_api_token_resource_permissions_type
				WHERE
					user_api_token_resource_permissions_type.permission_id = local_permission_id AND
					user_api_token_resource_permissions_type.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id AND
					user_api_token_resource_permissions_type.resource_permission_type = 'exclude'
			)
			/*
			We are basically doing:
				if super_admin OR
				in(include_list) OR
				(has exclude on permissionId AND not in(exclude_list))
			*/
			SELECT
				resource.*
			FROM
				resource
			WHERE
				/* Super admin: all resources in owned workspaces */
				EXISTS (
					SELECT
						1
					FROM
						super_admin_workspaces
					WHERE
						super_admin_workspaces.workspace_id = resource.workspace_id
				)
				/* Include: any role or API token explicitly grants this resource
				(also overrides exclude — include always wins) */
				OR EXISTS (
					SELECT
						1
					FROM
						included_resources
					WHERE
						included_resources.resource_id = resource.id AND
						included_resources.workspace_id = resource.workspace_id
				)
				/* Exclude: resource is in a workspace with an exclude-type
				permission and is not on any deny list */
				OR (
					EXISTS (
						SELECT
							1
						FROM
							exclude_workspaces
						WHERE
							exclude_workspaces.workspace_id = resource.workspace_id
					) AND NOT EXISTS (
						SELECT
							1
						FROM
							excluded_resources
						WHERE
							excluded_resources.resource_id = resource.id
					)
				);
		END;
		$$ LANGUAGE plpgsql;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
