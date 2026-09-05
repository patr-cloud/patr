//! Moves API-token evaluation in `RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID`
//! from the legacy declared-permission tables onto the token's own
//! `user_api_token_permission_binding` ceiling rows, intersected per-resource
//! with the owner's binding-derived grants.
//!
//! Every token arm resolves through its owner, matching the Rust loader:
//! ceiling rows intersect the owner's grants, and a token's super-admin row
//! counts only while its owner still holds the workspace.
//!
//! While the function is being rewritten anyway, it also stops returning
//! tombstoned resources. Nothing could act on one — the authorizer re-checks
//! `deleted IS NULL`, and every list endpoint filters its own table — but the
//! guarantee lived entirely in the callers, so a sixth list endpoint that
//! forgot would have leaked deleted rows to anyone holding a stale binding.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE OR REPLACE FUNCTION RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID(
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
					workspace.id AS workspace_id
				FROM
					user_api_token_workspace_super_admin sa
				INNER JOIN
					user_api_token
				ON
					user_api_token.token_id = sa.token_id
				INNER JOIN
					workspace
				ON
					workspace.id = sa.workspace_id AND
					workspace.super_admin_id = user_api_token.user_id
				WHERE
					sa.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
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
					workspace_actor
				ON
					workspace_actor.actor_type = 'user' AND
					workspace_actor.user_id = web_login.user_id
				INNER JOIN
					role_binding
				ON
					role_binding.actor_id = workspace_actor.id
				INNER JOIN
					role_permission
				ON
					role_permission.role_id = role_binding.role_id AND
					role_permission.permission_id = local_permission_id
				WHERE
					web_login.login_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
			),
			/* An API token's declared ceiling: its own (permission, scope) rows */
			token_ceiling AS (
				SELECT
					pb.workspace_id,
					pb.scope_id
				FROM
					user_api_token_permission_binding pb
				WHERE
					pb.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id AND
					pb.permission_id = local_permission_id
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
					workspace_actor
				ON
					workspace_actor.actor_type = 'user' AND
					workspace_actor.user_id = user_api_token.user_id
				INNER JOIN
					role_binding
				ON
					role_binding.actor_id = workspace_actor.id
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
				/* Tombstoned resources are nobody's, whatever the bindings say. The
				parenthesised disjunction matters: AND binds tighter than OR, so
				without it this would only narrow the super-admin arm. */
				resource.deleted IS NULL AND
				(
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
		END;
		$$ LANGUAGE plpgsql;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
