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
	// user's actor requires membership; a service account is its own actor.
	// One actor per principal per workspace.
	query!(
		r#"
		CREATE TABLE workspace_actor(
			id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			actor_type ACTOR_TYPE NOT NULL
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
			created_by UUID NOT NULL
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
			workspace_id UUID NOT NULL,
			actor_id UUID NOT NULL,
			actor_type ACTOR_TYPE NOT NULL
				GENERATED ALWAYS AS ('user') STORED
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
		ALTER TABLE workspace_user
		ADD CONSTRAINT workspace_user_pk
		PRIMARY KEY(user_id, workspace_id);
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
	// role_binding -> workspace_actor -> workspace_user.
	query!(
		r#"
		ALTER TABLE workspace_actor
			ADD CONSTRAINT workspace_actor_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// The actor is the identity role bindings point at; each kind of
	// principal points back at it. The generated `actor_type` here, paired
	// with the actor's UNIQUE(id, actor_type), stops a membership row
	// attaching to a non-user actor.
	query!(
		r#"
		ALTER TABLE workspace_user
			ADD CONSTRAINT workspace_user_uq_actor_id UNIQUE(actor_id),
			ADD CONSTRAINT workspace_user_fk_actor_id_actor_type
				FOREIGN KEY(actor_id, actor_type)
					REFERENCES workspace_actor(id, actor_type),
			ADD CONSTRAINT workspace_user_fk_actor_id_workspace_id
				FOREIGN KEY(actor_id, workspace_id)
					REFERENCES workspace_actor(id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Every FK pivots on workspace_id: the actor, the role, and the scope
	// must all live in the binding's workspace. No `deleted` in the scope
	// FKs — a binding onto a tombstoned resource is inert: the resource
	// lookup filters `deleted IS NULL`, and so does the authorizer.
	query!(
		r#"
		ALTER TABLE role_binding
			ADD CONSTRAINT role_binding_fk_actor_id_workspace_id
				FOREIGN KEY(actor_id, workspace_id) REFERENCES workspace_actor(id, workspace_id),
			ADD CONSTRAINT role_binding_fk_role_id_workspace_id
				FOREIGN KEY(role_id, workspace_id) REFERENCES role(id, workspace_id),
			ADD CONSTRAINT role_binding_fk_scope_id_workspace_id
				FOREIGN KEY(scope_id, workspace_id) REFERENCES resource(id, workspace_id),
			ADD CONSTRAINT role_binding_fk_created_by
				FOREIGN KEY(created_by) REFERENCES actor(id);
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
			Uuid::new_v4(),
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
			Uuid::new_v4(),
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
			local_client_type ACTOR_CLIENT_TYPE;
			local_login_type USER_LOGIN_TYPE;
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

			/* The kind of login decides where its grants come from, so only
			that kind's query runs. A login with no actor_client row matches
			no branch below and gets nothing. */
			SELECT
				actor_client.actor_client_type,
				user_login.login_type
			INTO
				local_client_type,
				local_login_type
			FROM
				actor_client
			LEFT JOIN
				user_login
			ON
				user_login.login_id = actor_client.id
			WHERE
				actor_client.id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id;

			/* Each branch collects the workspaces its grants touch first and
			only reads those workspaces' resources. Scope covers a resource
			when it is the resource itself or its whole workspace — two OR
			terms, never IN (NULL semantics); a third term arrives when
			projects land. Tombstoned resources are nobody's, whatever the
			grants say. */
			IF local_client_type = 'runner' THEN
				/* A runner holds no bindings. Its grants are fixed, and must
				match the map its token loads: read access across its own
				workspace, and view and edit on itself. Its actor_client row
				outlives it, so a deleted runner is turned away on its own
				row. */
				RETURN QUERY
				SELECT
					resource.*
				FROM
					runner
				INNER JOIN
					resource
				ON
					resource.workspace_id = runner.workspace_id
				WHERE
					runner.id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id AND
					runner.deleted IS NULL AND
					resource.deleted IS NULL AND (
						permission_name IN (
							'containerRegistryRepository::pull',
							'containerRegistryRepository::view',
							'deployment::view',
							'domain::view',
							'managedURL::view',
							'secret::view'
						) OR (
							permission_name IN (
								'runner::edit',
								'runner::view'
							) AND
							resource.id = runner.id
						)
					);
			ELSIF local_client_type = 'service_account' THEN
				/* A service account authenticates as itself, so one id is its
				login, its actor_client registration and its actor: there is
				no membership row to go through. */
				RETURN QUERY
				WITH bindings AS (
					SELECT
						role_binding.workspace_id,
						role_binding.scope_id
					FROM
						service_account
					INNER JOIN
						role_binding
					ON
						role_binding.actor_id = service_account.id
					INNER JOIN
						role_permission
					ON
						role_permission.role_id = role_binding.role_id AND
						role_permission.permission_id = local_permission_id
					WHERE
						service_account.id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id AND
						service_account.deleted IS NULL
				)
				SELECT
					resource.*
				FROM
					(
						SELECT DISTINCT
							bindings.workspace_id
						FROM
							bindings
					) AS candidate_workspaces
				INNER JOIN
					resource
				ON
					resource.workspace_id = candidate_workspaces.workspace_id
				WHERE
					resource.deleted IS NULL AND
					EXISTS (
						SELECT
							1
						FROM
							bindings
						WHERE
							bindings.workspace_id = resource.workspace_id AND
							(
								bindings.scope_id = resource.id OR
								bindings.scope_id = bindings.workspace_id
							)
					);
			ELSIF local_login_type IN ('web_login', 'oauth_login') THEN
				RETURN QUERY
				/* Workspaces where this login's user has super admin access */
				WITH super_admin_workspaces AS (
					SELECT
						workspace.id AS workspace_id
					FROM
						user_login
					INNER JOIN
						workspace
					ON
						workspace.super_admin_id = user_login.user_id
					WHERE
						user_login.login_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
				),
				/* Bindings carrying this permission, through the user's
				membership */
				bindings AS (
					SELECT
						role_binding.workspace_id,
						role_binding.scope_id
					FROM
						user_login
					INNER JOIN
						workspace_user
					ON
						workspace_user.user_id = user_login.user_id
					INNER JOIN
						role_binding
					ON
						role_binding.actor_id = workspace_user.actor_id
					INNER JOIN
						role_permission
					ON
						role_permission.role_id = role_binding.role_id AND
						role_permission.permission_id = local_permission_id
					WHERE
						user_login.login_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
				)
				SELECT
					resource.*
				FROM
					(
						SELECT DISTINCT
							candidate.workspace_id
						FROM
							(
								SELECT
									super_admin_workspaces.workspace_id
								FROM
									super_admin_workspaces
								UNION ALL
								SELECT
									bindings.workspace_id
								FROM
									bindings
							) AS candidate(workspace_id)
					) AS candidate_workspaces
				INNER JOIN
					resource
				ON
					resource.workspace_id = candidate_workspaces.workspace_id
				WHERE
					/* The parenthesised disjunction matters: AND binds tighter
					than OR, so without it this would only narrow the
					super-admin arm. */
					resource.deleted IS NULL AND (
						EXISTS (
							SELECT
								1
							FROM
								super_admin_workspaces
							WHERE
								super_admin_workspaces.workspace_id = resource.workspace_id
						) OR EXISTS (
							SELECT
								1
							FROM
								bindings
							WHERE
								bindings.workspace_id = resource.workspace_id AND
								(
									bindings.scope_id = resource.id OR
									bindings.scope_id = bindings.workspace_id
								)
						)
					);
			ELSIF local_login_type = 'api_token' THEN
				RETURN QUERY
				/* Workspaces where this token has super admin access */
				WITH super_admin_workspaces AS (
					SELECT
						user_api_token_workspace_super_admin.workspace_id
					FROM
						user_api_token_workspace_super_admin
					WHERE
						user_api_token_workspace_super_admin.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
				),
				/* An API token's declared ceiling: its own (permission, scope)
				rows */
				token_ceiling AS (
					SELECT
						user_api_token_permission_binding.workspace_id,
						user_api_token_permission_binding.scope_id
					FROM
						user_api_token_permission_binding
					WHERE
						user_api_token_permission_binding.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id AND
						user_api_token_permission_binding.permission_id = local_permission_id
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
						workspace_user
					ON
						workspace_user.user_id = user_api_token.user_id
					INNER JOIN
						role_binding
					ON
						role_binding.actor_id = workspace_user.actor_id
					INNER JOIN
						role_permission
					ON
						role_permission.role_id = role_binding.role_id AND
						role_permission.permission_id = local_permission_id
					WHERE
						user_api_token.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
					UNION ALL
					/* A super admin holds every permission in their workspace
					without holding any role binding, so a token they own is
					capped only by its own ceiling. Projecting the workspace id
					into scope_id reuses the "scope is the whole workspace"
					convention the filter below applies. */
					SELECT
						workspace.id AS workspace_id,
						workspace.id AS scope_id
					FROM
						user_api_token
					INNER JOIN
						workspace
					ON
						workspace.super_admin_id = user_api_token.user_id
					WHERE
						user_api_token.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
				)
				/* Every grant has to be within the ceiling, so the ceiling's
				workspaces are the only ones the owner's grants can reach */
				SELECT
					resource.*
				FROM
					(
						SELECT DISTINCT
							candidate.workspace_id
						FROM
							(
								SELECT
									super_admin_workspaces.workspace_id
								FROM
									super_admin_workspaces
								UNION ALL
								SELECT
									token_ceiling.workspace_id
								FROM
									token_ceiling
							) AS candidate(workspace_id)
					) AS candidate_workspaces
				INNER JOIN
					resource
				ON
					resource.workspace_id = candidate_workspaces.workspace_id
				WHERE
					resource.deleted IS NULL AND (
						EXISTS (
							SELECT
								1
							FROM
								super_admin_workspaces
							WHERE
								super_admin_workspaces.workspace_id = resource.workspace_id
						) OR (
							EXISTS (
								SELECT
									1
								FROM
									token_ceiling
								WHERE
									token_ceiling.workspace_id = resource.workspace_id AND
									(
										token_ceiling.scope_id = resource.id OR
										token_ceiling.scope_id = token_ceiling.workspace_id
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
						)
					);
			END IF;
		END;
		$$ LANGUAGE plpgsql;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
