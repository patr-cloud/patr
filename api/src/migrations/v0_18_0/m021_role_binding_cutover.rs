//! Cuts evaluation and membership over to the role-binding model.
//!
//! - `RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID` is rewritten over `role_binding ⋈ role_permission`.
//!   The include/exclude CTEs disappear — and with them the cross-workspace exclude leak
//!   (`excluded_resources` used to drop its workspace column). The token arm keeps reading the
//!   legacy declared tables (until the token DTOs move to ceiling rows), intersected per-resource
//!   with the owner's binding-derived grants now that the auth-path write-back is gone.
//! - `workspace_user` collapses to pure membership: `role_id` goes, the PK becomes `(user_id,
//!   workspace_id)`, and its two secondary indexes — now a strict prefix and an exact duplicate of
//!   the PK — go with it.
//! - `actor` finally gets its membership FK, deferred since the table was created because the old
//!   PK still carried `role_id`.
//! - Pending invites gain a `scope_id`, expanded by the same three rules the backfill used.
//!   Acceptance mints a binding straight from it, so an invite issued against a scoped role grants
//!   exactly that scope rather than the whole workspace.

/// A scope for every invite role row.
mod fill_user_invite_scope;
/// The `scope_id` column on `workspace_user_invite_role`.
mod user_invite_add_scope;

use self::{
	fill_user_invite_scope::fill_user_invite_scope,
	user_invite_add_scope::user_invite_add_scope,
};
use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	user_invite_add_scope(&mut *connection).await?;
	fill_user_invite_scope(&mut *connection).await?;

	sqlx::query(
		r#"
		REPLACE FUNCTION RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID(
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
					workspace_user
				ON
					workspace_user.user_id = web_login.user_id
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
					web_login.login_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
			),
			/* An API token's declared grants, from the legacy snapshot tables
			(interim until the token DTOs move to ceiling rows): granted on a
			resource when the include list names it, or the workspace has an
			exclude-type entry whose list doesn't */
			token_included AS (
				SELECT
					user_api_token_resource_permissions_include.resource_id,
					user_api_token_resource_permissions_include.workspace_id
				FROM
					user_api_token_resource_permissions_include
				WHERE
					user_api_token_resource_permissions_include.permission_id = local_permission_id AND
					user_api_token_resource_permissions_include.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
			),
			token_excluded AS (
				SELECT
					user_api_token_resource_permissions_exclude.resource_id
				FROM
					user_api_token_resource_permissions_exclude
				WHERE
					user_api_token_resource_permissions_exclude.permission_id = local_permission_id AND
					user_api_token_resource_permissions_exclude.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
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
				) OR EXISTS (
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
				) OR (
					(
						EXISTS (
							SELECT
								1
							FROM
								token_included
							WHERE
								token_included.workspace_id = resource.workspace_id AND
								token_included.resource_id = resource.id
						) OR (
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
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user DROP CONSTRAINT workspace_user_pk;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user DROP CONSTRAINT workspace_user_fk_role_id_workspace_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user DROP COLUMN role_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DELETE FROM workspace_user a
		USING workspace_user b
		WHERE
			a.ctid < b.ctid AND
			a.user_id = b.user_id AND
			a.workspace_id = b.workspace_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
		ADD CONSTRAINT workspace_user_pk
		PRIMARY KEY(user_id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP INDEX workspace_user_idx_user_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP INDEX workspace_user_idx_user_id_workspace_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Both deferred from the actor migration: until the rows above collapsed,
	// every role of a membership shared its actor id, and the handlers only
	// start writing the column here.
	sqlx::query(
		r#"
		ALTER TABLE workspace_user
		ALTER COLUMN actor_id SET NOT NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
		ADD CONSTRAINT workspace_user_uq_actor_id UNIQUE(actor_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
