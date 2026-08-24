//! Cuts evaluation and membership over to the role-binding model.
//!
//! - `RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID` is rewritten over `role_binding ⋈
//!   role_permission`. The include/exclude CTEs disappear — and with them the
//!   cross-workspace exclude leak (`excluded_resources` used to drop its
//!   workspace column). The token arm keeps reading the legacy declared
//!   tables (until the token DTOs move to ceiling rows), intersected
//!   per-resource with the owner's binding-derived grants now that the
//!   auth-path write-back is gone.
//! - `workspace_user_invite_role.scope_id` becomes NOT NULL and rejoins the
//!   primary key (every row was given a scope by the backfill).
//! - `workspace_user` collapses to pure membership: `role_id` goes, the PK
//!   becomes `(user_id, workspace_id)`, and its two secondary indexes — now a
//!   strict prefix and an exact duplicate of the PK — go with it.
//! - `actor` finally gets its membership FK, deferred since the table was
//!   created because the old PK still carried `role_id`.

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
					user_api_token_workspace_super_admin.workspace_id
				FROM
					user_api_token_workspace_super_admin
				WHERE
					user_api_token_workspace_super_admin.token_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
			),
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
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite_role
		ALTER COLUMN scope_id SET NOT NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP INDEX workspace_user_invite_role_uq_invite_id_role_id_scope_id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user_invite_role
		ADD CONSTRAINT workspace_user_invite_role_pk
		PRIMARY KEY(invite_id, role_id, scope_id);
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

	sqlx::query(
		r#"
		ALTER TABLE actor
		ADD CONSTRAINT actor_fk_user_id_workspace_id
		FOREIGN KEY(user_id, workspace_id) REFERENCES workspace_user(user_id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
