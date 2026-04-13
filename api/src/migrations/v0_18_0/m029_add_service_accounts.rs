//! Adds service accounts: a non-human identity for runners and automation.
//!
//! A service account wears three hats that all key off one id — it is a
//! `resource` (so it can be permission-gated like anything else), a
//! `workspace_actor` (so role bindings can be granted on it) and an
//! `actor_client` (so it can authenticate and leave an audit trail). Reusing
//! the id rather than minting three is the same trick `user_login` already
//! uses to register into `actor_client`, and it is what lets the permission
//! function reach a service account's bindings without a membership row.
//!
//! Unlike a user, a service account is workspace-scoped by construction, so
//! its actor is one-to-one with the account and needs no join table. It is
//! never a workspace super admin — workspaces are owned by humans.
//!
//! Both enums are recreated rather than extended with `ALTER TYPE ... ADD
//! VALUE`: every migration runs inside one transaction, and Postgres refuses
//! to *use* a value added to a pre-existing enum in the transaction that added
//! it. Swapping the type out — drop the dependent generated columns and their
//! foreign keys, retype the supertype column, put them back — is the only
//! form that works without committing mid-migration.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// --- WORKSPACE_ACTOR_TYPE gains 'service_account' -----------------------

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
		DROP CONSTRAINT workspace_user_fk_actor_id_actor_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
		DROP COLUMN actor_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TYPE WORKSPACE_ACTOR_TYPE RENAME TO WORKSPACE_ACTOR_TYPE_OLD;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TYPE WORKSPACE_ACTOR_TYPE AS ENUM(
			'user',
			'service_account'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_actor
		ALTER COLUMN actor_type TYPE WORKSPACE_ACTOR_TYPE
		USING actor_type::TEXT::WORKSPACE_ACTOR_TYPE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TYPE WORKSPACE_ACTOR_TYPE_OLD;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
		ADD COLUMN actor_type WORKSPACE_ACTOR_TYPE NOT NULL
		GENERATED ALWAYS AS ('user') STORED;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
		ADD CONSTRAINT workspace_user_fk_actor_id_actor_type
		FOREIGN KEY(actor_id, actor_type) REFERENCES workspace_actor(id, actor_type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// --- ACTOR_CLIENT_TYPE gains 'service_account' --------------------------

	sqlx::query(
		r#"
		ALTER TABLE user_login
		DROP CONSTRAINT user_login_fk_login_id_actor_client_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_login
		DROP COLUMN actor_client_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TYPE ACTOR_CLIENT_TYPE RENAME TO ACTOR_CLIENT_TYPE_OLD;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TYPE ACTOR_CLIENT_TYPE AS ENUM(
			'user_login',
			'service_account'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE actor_client
		ALTER COLUMN actor_client_type TYPE ACTOR_CLIENT_TYPE
		USING actor_client_type::TEXT::ACTOR_CLIENT_TYPE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TYPE ACTOR_CLIENT_TYPE_OLD;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_login
		ADD COLUMN actor_client_type ACTOR_CLIENT_TYPE NOT NULL
		GENERATED ALWAYS AS ('user_login') STORED;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_login
		ADD CONSTRAINT user_login_fk_login_id_actor_client_type
		FOREIGN KEY(login_id, actor_client_type) REFERENCES actor_client(id, actor_client_type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE service_account(
			id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			name VARCHAR(100) NOT NULL,
			description TEXT,
			token_hash TEXT NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			deleted TIMESTAMPTZ,
			actor_type WORKSPACE_ACTOR_TYPE NOT NULL
				GENERATED ALWAYS AS ('service_account') STORED,
			actor_client_type ACTOR_CLIENT_TYPE NOT NULL
				GENERATED ALWAYS AS ('service_account') STORED
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE service_account
			ADD CONSTRAINT service_account_pk PRIMARY KEY(id),
			ADD CONSTRAINT service_account_uq_id_workspace_id UNIQUE(id, workspace_id);
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
			ADD CONSTRAINT service_account_fk_id_workspace_id_deleted
				FOREIGN KEY(id, workspace_id, deleted)
					REFERENCES resource(id, workspace_id, deleted),
			ADD CONSTRAINT service_account_fk_id_actor_type
				FOREIGN KEY(id, actor_type)
					REFERENCES workspace_actor(id, actor_type),
			ADD CONSTRAINT service_account_fk_id_actor_workspace_id
				FOREIGN KEY(id, workspace_id)
					REFERENCES workspace_actor(id, workspace_id),
			ADD CONSTRAINT service_account_fk_id_actor_client_type
				FOREIGN KEY(id, actor_client_type)
					REFERENCES actor_client(id, actor_client_type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		INSERT INTO
			resource_type(id, name, description)
		VALUES
			(
				GEN_RANDOM_UUID(),
				'serviceAccount',
				'A non-human identity within a workspace, used to authenticate runners and other automated processes. It holds a single token and is granted access through role bindings like any other actor.'
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	for (name, description) in [
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
				(GEN_RANDOM_UUID(), $1, $2);
			"#,
		)
		.bind(name)
		.bind(description)
		.execute(&mut *connection)
		.await?;
	}

	// Dropped, not replaced: the function is created by m014 and rewritten by
	// m021 and m022, so it must already exist. If it doesn't, something
	// upstream is broken and the migration should fail here rather than
	// quietly conjure one.
	sqlx::query(
		r#"
		DROP FUNCTION RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID(UUID, TEXT);
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
			/* Bindings carrying this permission for a client that acts as
			itself: a web login resolves through its user's membership, a
			service account is its own actor. An API token holds none of
			these — its arm is the ceiling further down. */
			actor_bindings AS (
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
				UNION ALL
				/* A service account authenticates as itself, so one id is its
				login, its actor_client registration and its actor: there is no
				membership row to go through. */
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
			),
			/* An API token's declared ceiling: its own (permission, scope) rows */
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
				/* A super admin holds every permission in their workspace without
				holding any role binding, so a token they own is capped only by its
				own ceiling. Projecting the workspace id into scope_id reuses the
				"scope is the whole workspace" convention the filter below applies. */
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
							actor_bindings
						WHERE
							actor_bindings.workspace_id = resource.workspace_id AND
							(
								actor_bindings.scope_id = resource.id OR
								actor_bindings.scope_id = actor_bindings.workspace_id
							)
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
		END;
		$$ LANGUAGE plpgsql;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
