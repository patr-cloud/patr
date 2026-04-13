//! Add service accounts: a non-human identity for runners and automation.
//!
//! - Creates `service_account` table (with token_hash for single-token auth)
//! - Creates `service_account_role` table (role assignments, enforcing
//!   workspace match)
//! - Adds `UNIQUE(id, owner_id)` on `role` for composite FK targets
//! - Updates `workspace_user` FK to enforce role belongs to same workspace
//! - Drops and recreates `RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID` with SA
//!   branches
//! - Inserts new permission and resource_type rows for service accounts

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// Create service_account table
	sqlx::query(
		r#"
		CREATE TABLE service_account(
			id UUID NOT NULL,
			name VARCHAR(100) NOT NULL,
			workspace_id UUID NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			description TEXT,
			token_hash TEXT NOT NULL,
			deleted TIMESTAMPTZ
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE service_account
			ADD CONSTRAINT service_account_pk
				PRIMARY KEY(id),
			ADD CONSTRAINT service_account_uq_id_workspace_id
				UNIQUE(id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE UNIQUE INDEX
			service_account_uq_workspace_id_name
		ON
			service_account(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE service_account
			ADD CONSTRAINT service_account_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id),
			ADD CONSTRAINT service_account_fk_id_workspace_id
				FOREIGN KEY(id, workspace_id, deleted)
					REFERENCES resource(id, owner_id, deleted);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Create service_account_role table
	sqlx::query(
		r#"
		CREATE TABLE service_account_role(
			service_account_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			role_id UUID NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE service_account_role
		ADD CONSTRAINT service_account_role_pk
		PRIMARY KEY(service_account_id, role_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Add UNIQUE(id, owner_id) on role for composite FK targets
	sqlx::query(
		r#"
		ALTER TABLE role
		ADD CONSTRAINT role_uq_id_owner_id
		UNIQUE(id, owner_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// service_account_role FKs: enforce workspace match
	sqlx::query(
		r#"
		ALTER TABLE service_account_role
			ADD CONSTRAINT service_account_role_fk_service_account_id_workspace_id
				FOREIGN KEY(service_account_id, workspace_id)
					REFERENCES service_account(id, workspace_id),
			ADD CONSTRAINT service_account_role_fk_role_id_workspace_id
				FOREIGN KEY(role_id, workspace_id)
					REFERENCES role(id, owner_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Update workspace_user FK to enforce role belongs to same workspace
	sqlx::query(
		r#"
		ALTER TABLE workspace_user
		DROP CONSTRAINT IF EXISTS workspace_user_fk_role_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
		ADD CONSTRAINT workspace_user_fk_role_id_workspace_id
			FOREIGN KEY(role_id, workspace_id)
				REFERENCES role(id, owner_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Insert ServiceAccount resource type
	sqlx::query(
		r#"
		INSERT INTO
			resource_type(id, name, description)
		VALUES
			(gen_random_uuid(), 'serviceAccount', 'A service account within a workspace. A service account is a non-human identity that can be used to authenticate runners and other automated processes. It has a single token and can be assigned roles within its workspace.');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Insert ServiceAccount permissions
	for permission in [
		(
			"serviceAccount::create",
			"This permission allows the user to create a new service account in a workspace.",
		),
		(
			"serviceAccount::view",
			"This permission allows the user to view the service account and its details.",
		),
		(
			"serviceAccount::edit",
			"This permission allows the user to edit the service account, but not delete it or create a new one.",
		),
		(
			"serviceAccount::delete",
			"This permission allows the user to delete the service account, but not create a new one, view it, or edit it.",
		),
		(
			"serviceAccount::regenerateToken",
			"This permission allows the user to regenerate the service account token.",
		),
	] {
		sqlx::query(
			r#"
			INSERT INTO
				permission(id, name, description)
			VALUES
				(gen_random_uuid(), $1, $2);
			"#,
		)
		.bind(permission.0)
		.bind(permission.1)
		.execute(&mut *connection)
		.await?;
	}

	// Drop and recreate RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID
	sqlx::query(
		r#"
		DROP FUNCTION IF EXISTS RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID(UUID, TEXT);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE FUNCTION RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID(
			login_id UUID,
			permission_name TEXT
		) RETURNS TABLE(
			id UUID,
			resource_type_id UUID,
			owner_id UUID,
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
			/* Resources explicitly granted via include lists */
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
				UNION ALL
				/* Service account role-based include permissions */
				SELECT
					role_resource_permissions_include.resource_id,
					service_account.workspace_id
				FROM
					service_account_role
				INNER JOIN
					service_account
				ON
					service_account.id = service_account_role.service_account_id
				INNER JOIN
					role_resource_permissions_include
				ON
					role_resource_permissions_include.role_id = service_account_role.role_id AND
					role_resource_permissions_include.permission_id = local_permission_id
				WHERE
					service_account_role.service_account_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
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
				UNION ALL
				/* Service account role-based exclude permissions */
				SELECT
					role_resource_permissions_exclude.resource_id
				FROM
					service_account_role
				INNER JOIN
					role_resource_permissions_exclude
				ON
					role_resource_permissions_exclude.role_id = service_account_role.role_id AND
					role_resource_permissions_exclude.permission_id = local_permission_id
				WHERE
					service_account_role.service_account_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
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
				UNION ALL
				/* Service account workspaces with exclude-type permission */
				SELECT
					service_account.workspace_id
				FROM
					service_account_role
				INNER JOIN
					service_account
				ON
					service_account.id = service_account_role.service_account_id
				INNER JOIN
					role_resource_permissions_type
				ON
					role_resource_permissions_type.role_id = service_account_role.role_id AND
					role_resource_permissions_type.permission_id = local_permission_id AND
					role_resource_permissions_type.permission_type = 'exclude'
				WHERE
					service_account_role.service_account_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
			)
			SELECT
				resource.*
			FROM
				resource
			WHERE
				EXISTS (
					SELECT 1
					FROM super_admin_workspaces
					WHERE super_admin_workspaces.workspace_id = resource.owner_id
				)
				OR EXISTS (
					SELECT 1
					FROM included_resources
					WHERE
						included_resources.resource_id = resource.id AND
						included_resources.workspace_id = resource.owner_id
				)
				OR (
					EXISTS (
						SELECT 1
						FROM exclude_workspaces
						WHERE exclude_workspaces.workspace_id = resource.owner_id
					) AND NOT EXISTS (
						SELECT 1
						FROM excluded_resources
						WHERE excluded_resources.resource_id = resource.id
					)
				);
		END;
		$$ LANGUAGE plpgsql;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
