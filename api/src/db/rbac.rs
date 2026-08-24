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
			is_immutable BOOLEAN NOT NULL DEFAULT FALSE
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
	// user's actor requires membership; a service account's (future) requires
	// its workspace-owned row. One actor per principal per workspace.
	query!(
		r#"
		CREATE TYPE ACTOR_TYPE AS ENUM(
			'user',
			'service_account'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE actor(
			id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			actor_type ACTOR_TYPE NOT NULL,
			user_id UUID,
			service_account_id UUID
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// The only place a permission target appears. Scope is a resource id;
	// workspace-wide = scope_id = workspace_id (workspaces self-own their
	// resource row), so there is no scope-type column.
	query!(
		r#"
		CREATE TABLE role_binding(
			id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			actor_id UUID NOT NULL,
			role_id UUID NOT NULL,
			scope_id UUID NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			created_by UUID
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// A token's own (role, scope) ceiling — independent of the owner's
	// bindings. Effective permissions are the ceiling intersected with the
	// owner's current permissions at auth time.
	query!(
		r#"
		CREATE TABLE api_token_role_binding(
			token_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			role_id UUID NOT NULL,
			scope_id UUID NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Pure membership; role grants live in role_binding
	query!(
		r#"
		CREATE TABLE workspace_user(
			user_id UUID NOT NULL,
			workspace_id UUID NOT NULL
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
		ALTER TABLE actor
			ADD CONSTRAINT actor_pk PRIMARY KEY(id),
			ADD CONSTRAINT actor_uq_id_workspace_id UNIQUE(id, workspace_id),
			ADD CONSTRAINT actor_uq_user_id_workspace_id UNIQUE(user_id, workspace_id),
			ADD CONSTRAINT actor_uq_service_account_id_workspace_id
				UNIQUE(service_account_id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role_binding
			ADD CONSTRAINT role_binding_pk PRIMARY KEY(id),
			ADD CONSTRAINT role_binding_uq_actor_id_role_id_scope_id
				UNIQUE(actor_id, role_id, scope_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_binding_idx_actor_id
		ON
			role_binding
		(actor_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_binding_idx_scope_id
		ON
			role_binding
		(scope_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE api_token_role_binding
		ADD CONSTRAINT api_token_role_binding_pk
		PRIMARY KEY(token_id, role_id, scope_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_user
		ADD CONSTRAINT workspace_user_pk
		PRIMARY KEY(user_id, workspace_id);
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

	// A user's actor requires membership; the FK chain is
	// role_binding -> actor -> workspace_user.
	query!(
		r#"
		ALTER TABLE actor
			ADD CONSTRAINT actor_fk_user_id_workspace_id
				FOREIGN KEY(user_id, workspace_id)
					REFERENCES workspace_user(user_id, workspace_id),
			ADD CONSTRAINT actor_chk_type_matches_columns CHECK(
				(
					actor_type = 'user' AND
					user_id IS NOT NULL AND
					service_account_id IS NULL
				) OR (
					actor_type = 'service_account' AND
					service_account_id IS NOT NULL AND
					user_id IS NULL
				)
			),
			ADD CONSTRAINT actor_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Every FK pivots on workspace_id: the actor, the role, and the scope
	// must all live in the binding's workspace. No `deleted` in the scope
	// FKs — a binding onto a tombstoned resource is inert, the authorizer
	// re-checks `deleted IS NULL`.
	query!(
		r#"
		ALTER TABLE role_binding
			ADD CONSTRAINT role_binding_fk_actor_id_workspace_id
				FOREIGN KEY(actor_id, workspace_id) REFERENCES actor(id, workspace_id),
			ADD CONSTRAINT role_binding_fk_role_id_workspace_id
				FOREIGN KEY(role_id, workspace_id) REFERENCES role(id, workspace_id),
			ADD CONSTRAINT role_binding_fk_scope_id_workspace_id
				FOREIGN KEY(scope_id, workspace_id) REFERENCES resource(id, workspace_id),
			ADD CONSTRAINT role_binding_fk_created_by
				FOREIGN KEY(created_by) REFERENCES "user"(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE api_token_role_binding
			ADD CONSTRAINT api_token_role_binding_fk_token_id
				FOREIGN KEY(token_id) REFERENCES user_api_token(token_id),
			ADD CONSTRAINT api_token_role_binding_fk_role_id_workspace_id
				FOREIGN KEY(role_id, workspace_id) REFERENCES role(id, workspace_id),
			ADD CONSTRAINT api_token_role_binding_fk_scope_id_workspace_id
				FOREIGN KEY(scope_id, workspace_id) REFERENCES resource(id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_user
			ADD CONSTRAINT workspace_user_fk_user_id
				FOREIGN KEY(user_id) REFERENCES "user"(id),
			ADD CONSTRAINT workspace_user_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id);
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
			/* Bindings carrying this permission: the user's own for web
			logins; the token's ceiling intersected with the owner's grants
			for API tokens */
			/* The login's own bindings (empty for API tokens) */
			user_bindings AS (
				SELECT
					role_binding.workspace_id,
					role_binding.scope_id
				FROM
					web_login
				INNER JOIN
					actor
				ON
					actor.actor_type = 'user' AND
					actor.user_id = web_login.user_id
				INNER JOIN
					role_binding
				ON
					role_binding.actor_id = actor.id
				INNER JOIN
					role_permission
				ON
					role_permission.role_id = role_binding.role_id AND
					role_permission.permission_id = local_permission_id
				WHERE
					web_login.login_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
			),
			/* An API token's declared grants, from the legacy snapshot tables
			(interim until the token DTOs move to ceiling rows): granted on a
			resource when the include list names it, or the workspace has an
			exclude-type entry whose list doesn't */
			token_included AS (
				SELECT
					inc.workspace_id,
					inc.resource_id
				FROM
					user_api_token_resource_permissions_include inc
				WHERE
					inc.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id AND
					inc.permission_id = local_permission_id
			),
			token_excluded AS (
				SELECT
					exc.resource_id
				FROM
					user_api_token_resource_permissions_exclude exc
				WHERE
					exc.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id AND
					exc.permission_id = local_permission_id
			),
			token_exclude_workspaces AS (
				SELECT
					t.workspace_id
				FROM
					user_api_token_resource_permissions_type t
				WHERE
					t.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id AND
					t.permission_id = local_permission_id AND
					t.resource_permission_type = 'exclude'
			),
			/* The token owner's own grants; effective = ceiling ∩ owner,
			intersected per resource below */
			token_owner_grants AS (
				SELECT
					role_binding.workspace_id,
					role_binding.scope_id
				FROM
					user_api_token
				INNER JOIN
					actor
				ON
					actor.actor_type = 'user' AND
					actor.user_id = user_api_token.user_id
				INNER JOIN
					role_binding
				ON
					role_binding.actor_id = actor.id
				INNER JOIN
					role_permission
				ON
					role_permission.role_id = role_binding.role_id AND
					role_permission.permission_id = local_permission_id
				WHERE
					user_api_token.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
			)
			/* Scope covers a resource when it is the resource itself or its
			whole workspace — two OR terms, never IN (NULL semantics); a
			third term arrives when projects land */
			SELECT
				resource.*
			FROM
				resource
			WHERE
				EXISTS (
					SELECT
						1
					FROM
						super_admin_workspaces
					WHERE
						super_admin_workspaces.workspace_id = resource.workspace_id
				)
				OR EXISTS (
					SELECT
						1
					FROM
						user_bindings
					WHERE
						user_bindings.workspace_id = resource.workspace_id AND
						(
							user_bindings.scope_id = resource.id OR
							user_bindings.scope_id = user_bindings.workspace_id
						)
				)
				OR (
					(
						EXISTS (
							SELECT
								1
							FROM
								token_included
							WHERE
								token_included.workspace_id = resource.workspace_id AND
								token_included.resource_id = resource.id
						)
						OR (
							EXISTS (
								SELECT
									1
								FROM
									token_exclude_workspaces
								WHERE
									token_exclude_workspaces.workspace_id = resource.workspace_id
							) AND NOT EXISTS (
								SELECT
									1
								FROM
									token_excluded
								WHERE
									token_excluded.resource_id = resource.id
							)
						)
					) AND EXISTS (
						SELECT
							1
						FROM
							token_owner_grants
						WHERE
							token_owner_grants.workspace_id = resource.workspace_id AND
							(
								token_owner_grants.scope_id = resource.id OR
								token_owner_grants.scope_id = token_owner_grants.workspace_id
							)
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
