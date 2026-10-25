//! Makes every runner its own identity.
//!
//! A runner becomes an actor like a user or a service account: it gets an
//! `actor` row of the new `'runner'` kind, and registers in `actor_client`
//! under a new `'runner'` kind as the owner of its own credential, so it leaves
//! an audit trail like any other client. It authenticates with its own token
//! (`patr_rn_…`), looked up by its SHA-256 in the new `runner.token_hash`. It
//! isn't a `workspace_actor`, so it can never be bound to a role: what it may
//! do is fixed in code, and `RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID` gains a
//! branch that matches it.
//!
//! `ACTOR_TYPE` and `ACTOR_CLIENT_TYPE` are recreated rather than extended,
//! for the reason `m029` gives: Postgres refuses to use a value added to an
//! existing enum in the transaction that added it. Every generated column of
//! either type (`actor_type` on `"user"`, `service_account` and
//! `workspace_user`, `actor_client_type` on `user_login` and
//! `service_account`) drops with its foreign keys around the swap and comes
//! back after, in the same order.
//!
//! Every runner, deleted ones too, gets its `actor` and its `actor_client`
//! before `runner` is pinned to them.
//!
//! The backfilled `token_hash` is the SHA-256 of a random value that is never
//! recorded, so no token can ever match it. An existing runner stops
//! connecting until its token is regenerated.
//!
//! The function is rewritten last. It now looks up the login's kind once and
//! runs only that kind's query, over only the workspaces its grants touch,
//! rather than filtering every resource in the database.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	// --- ACTOR_TYPE and ACTOR_CLIENT_TYPE gain 'runner' ---------------------

	sqlx::query(
		r#"
		ALTER TABLE "user"
			DROP CONSTRAINT user_fk_id_actor_type,
			DROP COLUMN actor_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE service_account
			DROP CONSTRAINT service_account_fk_id_actor_type,
			DROP CONSTRAINT service_account_fk_id_actor_type_actor,
			DROP CONSTRAINT service_account_fk_id_actor_client_type_id,
			DROP COLUMN actor_type,
			DROP COLUMN actor_client_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
			DROP CONSTRAINT workspace_user_fk_actor_id_actor_type,
			DROP COLUMN actor_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_login
			DROP CONSTRAINT user_login_fk_login_id_actor_client_type_user_id,
			DROP COLUMN actor_client_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TYPE ACTOR_TYPE RENAME TO ACTOR_TYPE_OLD;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TYPE ACTOR_TYPE AS ENUM(
			'user',
			'service_account',
			'runner'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE actor
		ALTER COLUMN actor_type TYPE ACTOR_TYPE
		USING actor_type::TEXT::ACTOR_TYPE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_actor
		ALTER COLUMN actor_type TYPE ACTOR_TYPE
		USING actor_type::TEXT::ACTOR_TYPE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		DROP TYPE ACTOR_TYPE_OLD;
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
			'service_account',
			'runner'
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
		ALTER TABLE "user"
			ADD COLUMN actor_type ACTOR_TYPE NOT NULL
				GENERATED ALWAYS AS ('user') STORED,
			ADD CONSTRAINT user_fk_id_actor_type
				FOREIGN KEY(id, actor_type) REFERENCES actor(id, actor_type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE service_account
			ADD COLUMN actor_type ACTOR_TYPE NOT NULL
				GENERATED ALWAYS AS ('service_account') STORED,
			ADD COLUMN actor_client_type ACTOR_CLIENT_TYPE NOT NULL
				GENERATED ALWAYS AS ('service_account') STORED,
			ADD CONSTRAINT service_account_fk_id_actor_type
				FOREIGN KEY(id, actor_type)
					REFERENCES workspace_actor(id, actor_type),
			ADD CONSTRAINT service_account_fk_id_actor_type_actor
				FOREIGN KEY(id, actor_type) REFERENCES actor(id, actor_type),
			ADD CONSTRAINT service_account_fk_id_actor_client_type_id
				FOREIGN KEY(id, actor_client_type, id)
					REFERENCES actor_client(id, actor_client_type, actor_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_user
			ADD COLUMN actor_type ACTOR_TYPE NOT NULL
				GENERATED ALWAYS AS ('user') STORED,
			ADD CONSTRAINT workspace_user_fk_actor_id_actor_type
				FOREIGN KEY(actor_id, actor_type)
					REFERENCES workspace_actor(id, actor_type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_login
			ADD COLUMN actor_client_type ACTOR_CLIENT_TYPE NOT NULL
				GENERATED ALWAYS AS ('user_login') STORED,
			ADD CONSTRAINT user_login_fk_login_id_actor_client_type_user_id
				FOREIGN KEY(login_id, actor_client_type, user_id)
					REFERENCES actor_client(id, actor_client_type, actor_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// --- runner identity ----------------------------------------------------

	sqlx::query(
		r#"
		ALTER TABLE runner
		ADD COLUMN token_hash TEXT;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Random per row and never recorded, so no token can match it.
	sqlx::query(
		r#"
		UPDATE
			runner
		SET
			token_hash = ENCODE(SHA256(GEN_RANDOM_UUID()::TEXT::BYTEA), 'hex');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE runner
		ALTER COLUMN token_hash SET NOT NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// Deleted runners too: actors are never deleted, and their audit rows
	// will point at them.
	sqlx::query(
		r#"
		INSERT INTO
			actor(id, actor_type, created)
		SELECT
			runner.id,
			'runner',
			resource.created
		FROM
			runner
		INNER JOIN
			resource
		ON
			resource.id = runner.id;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		INSERT INTO
			actor_client(id, actor_client_type, actor_id)
		SELECT
			runner.id,
			'runner',
			runner.id
		FROM
			runner;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE runner
			ADD COLUMN actor_type ACTOR_TYPE NOT NULL
				GENERATED ALWAYS AS ('runner') STORED,
			ADD COLUMN actor_client_type ACTOR_CLIENT_TYPE NOT NULL
				GENERATED ALWAYS AS ('runner') STORED,
			ADD CONSTRAINT runner_fk_id_actor_type
				FOREIGN KEY(id, actor_type) REFERENCES actor(id, actor_type),
			ADD CONSTRAINT runner_fk_id_actor_client_type_id
				FOREIGN KEY(id, actor_client_type, id)
					REFERENCES actor_client(id, actor_client_type, actor_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE UNIQUE INDEX
			runner_uq_token_hash
		ON
			runner(token_hash);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// --- RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID -----------------------------

	// Dropped, not replaced, like `m029`: if it doesn't exist, something
	// upstream is broken and the migration should fail here.
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
					resource.deleted IS NULL AND
					(
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
			ELSIF local_login_type = 'web_login' THEN
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
				),
				/* Bindings carrying this permission, through the user's
				membership */
				bindings AS (
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
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
