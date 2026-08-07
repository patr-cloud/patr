//! Unify users and service accounts behind a common identity.
//!
//! Before this migration the two identity kinds were modelled in parallel:
//! `workspace_user` vs `service_account_role` for membership, and `user_login`
//! covering only human credentials. That left `audit_log.login_id` — a FK to
//! `user_login(login_id)` — unsatisfiable for a service account, so every
//! audited endpoint a service account called failed with a foreign key
//! violation.
//!
//! Three new pieces:
//!
//! - `identity(id, type)` — supertype of `"user"` and `service_account`. Both
//!   subtypes carry a generated `identity_type` column and a composite FK, so a
//!   user row can never be referenced where a service account is expected.
//! - `credential` — `user_login` generalised to any identity. A web session and
//!   an API token are still many-per-identity; a service account has exactly
//!   one, keyed on its own ID (the convention the auth path already used).
//! - `workspace_member(identity_id, workspace_id, role_id)` — replaces both
//!   membership tables.
//!
//! `audit_log` then only has to repoint its FK at `credential`, keeping
//! per-credential attribution ("which token did this") rather than collapsing
//! to the identity.

use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	create_identity_table(connection).await?;
	generalise_user_login_to_credential(connection).await?;
	unify_workspace_membership(connection).await?;
	repoint_audit_log(connection).await?;
	recreate_permission_functions(connection).await?;

	Ok(())
}

/// The identity supertype, backfilled from the existing users and service
/// accounts, with both subtypes pointing back at it.
async fn create_identity_table(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE TYPE IDENTITY_TYPE AS ENUM(
			'user',
			'service_account'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE TABLE identity(
			id UUID NOT NULL,
			type IDENTITY_TYPE NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE identity
			ADD CONSTRAINT identity_pk PRIMARY KEY(id),
			ADD CONSTRAINT identity_uq_id_type UNIQUE(id, type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		INSERT INTO
			identity(id, type)
		SELECT
			id, 'user'
		FROM
			"user";
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		INSERT INTO
			identity(id, type)
		SELECT
			id, 'service_account'
		FROM
			service_account;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// The generated column pins each subtype to its own discriminator, so the
	// composite FK makes "this ID is a user" unforgeable at the schema level.
	sqlx::query(
		r#"
		ALTER TABLE "user"
			ADD COLUMN identity_type IDENTITY_TYPE NOT NULL
				GENERATED ALWAYS AS ('user') STORED,
			ADD CONSTRAINT user_fk_id_identity_type
				FOREIGN KEY(id, identity_type) REFERENCES identity(id, type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE service_account
			ADD COLUMN identity_type IDENTITY_TYPE NOT NULL
				GENERATED ALWAYS AS ('service_account') STORED,
			ADD CONSTRAINT service_account_fk_id_identity_type
				FOREIGN KEY(id, identity_type) REFERENCES identity(id, type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// `user_login` becomes `credential`, keyed on identity rather than user, and
/// gains a row per service account.
async fn generalise_user_login_to_credential(
	connection: &mut DatabaseConnection,
) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE TYPE CREDENTIAL_TYPE AS ENUM(
			'web_login',
			'api_token',
			'service_account'
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// The dependents' `login_type` columns are GENERATED, so they can't be
	// retyped in place — drop them (and the FKs that use them) and rebuild
	// against CREDENTIAL_TYPE once the parent is converted.
	sqlx::query(
		r#"
		ALTER TABLE web_login
			DROP CONSTRAINT web_login_fk,
			DROP COLUMN login_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_api_token
			DROP CONSTRAINT user_api_token_token_id_user_id_login_type_fk,
			DROP COLUMN login_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_login
			ALTER COLUMN login_type TYPE CREDENTIAL_TYPE
				USING login_type::TEXT::CREDENTIAL_TYPE;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_login
			DROP CONSTRAINT user_login_fk_user_id,
			DROP CONSTRAINT user_login_uq_login_id_user_id,
			DROP CONSTRAINT user_login_uq_login_id_user_id_login_type;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(r#"ALTER TABLE user_login RENAME COLUMN login_id TO credential_id;"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(r#"ALTER TABLE user_login RENAME COLUMN user_id TO identity_id;"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(r#"ALTER TABLE user_login RENAME COLUMN login_type TO type;"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(r#"ALTER TABLE user_login RENAME CONSTRAINT user_login_pk TO credential_pk;"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(r#"ALTER TABLE user_login RENAME TO credential;"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(
		r#"
		ALTER TABLE credential
			ADD CONSTRAINT credential_fk_identity_id
				FOREIGN KEY(identity_id) REFERENCES identity(id),
			ADD CONSTRAINT credential_uq_credential_id_identity_id
				UNIQUE(credential_id, identity_id),
			ADD CONSTRAINT credential_uq_credential_id_identity_id_type
				UNIQUE(credential_id, identity_id, type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// A service account holds a single non-rotating credential and acts as its
	// own credential ID — the shape the auth path already assumed.
	sqlx::query(
		r#"
		INSERT INTO
			credential(credential_id, identity_id, type, created)
		SELECT
			id, id, 'service_account', created
		FROM
			service_account;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE UNIQUE INDEX
			credential_uq_identity_id_service_account
		ON
			credential(identity_id)
		WHERE
			type = 'service_account';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE web_login
			ADD COLUMN login_type CREDENTIAL_TYPE NOT NULL
				GENERATED ALWAYS AS ('web_login') STORED,
			ADD CONSTRAINT web_login_fk
				FOREIGN KEY(login_id, user_id, login_type)
					REFERENCES credential(credential_id, identity_id, type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE user_api_token
			ADD COLUMN login_type CREDENTIAL_TYPE NOT NULL
				GENERATED ALWAYS AS ('api_token') STORED,
			ADD CONSTRAINT user_api_token_token_id_user_id_login_type_fk
				FOREIGN KEY(token_id, user_id, login_type)
					REFERENCES credential(credential_id, identity_id, type);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(r#"DROP TYPE USER_LOGIN_TYPE;"#)
		.execute(&mut *connection)
		.await?;

	Ok(())
}

/// `workspace_user` + `service_account_role` collapse into one membership
/// table keyed on identity.
async fn unify_workspace_membership(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(
		r#"
		CREATE TABLE workspace_member(
			identity_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			role_id UUID NOT NULL
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		ALTER TABLE workspace_member
			ADD CONSTRAINT workspace_member_pk
				PRIMARY KEY(identity_id, workspace_id, role_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		INSERT INTO
			workspace_member(identity_id, workspace_id, role_id)
		SELECT
			user_id, workspace_id, role_id
		FROM
			workspace_user;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		INSERT INTO
			workspace_member(identity_id, workspace_id, role_id)
		SELECT
			service_account_id, workspace_id, role_id
		FROM
			service_account_role;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// The role/workspace composite FK is what stops a role from another
	// workspace being attached to a member of this one.
	sqlx::query(
		r#"
		ALTER TABLE workspace_member
			ADD CONSTRAINT workspace_member_fk_identity_id
				FOREIGN KEY(identity_id) REFERENCES identity(id),
			ADD CONSTRAINT workspace_member_fk_workspace_id
				FOREIGN KEY(workspace_id) REFERENCES workspace(id),
			ADD CONSTRAINT workspace_member_fk_role_id_workspace_id
				FOREIGN KEY(role_id, workspace_id) REFERENCES role(id, owner_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE INDEX
			workspace_member_idx_identity_id
		ON
			workspace_member(identity_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE INDEX
			workspace_member_idx_identity_id_workspace_id
		ON
			workspace_member(identity_id, workspace_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(r#"DROP TABLE workspace_user;"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(r#"DROP TABLE service_account_role;"#)
		.execute(&mut *connection)
		.await?;

	Ok(())
}

/// Audit rows now reference a credential of any identity kind, which is what
/// made service-account-authored actions fail the FK before.
async fn repoint_audit_log(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	sqlx::query(r#"ALTER TABLE audit_log DROP CONSTRAINT audit_log_login_id_fkey;"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(r#"ALTER TABLE audit_log RENAME COLUMN login_id TO credential_id;"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(
		r#"
		ALTER TABLE audit_log
			ADD CONSTRAINT audit_log_credential_id_fkey
				FOREIGN KEY(credential_id) REFERENCES credential(credential_id);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// `GENERATE_LOGIN_ID` and `RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID` both read
/// the tables that just moved. The permission function additionally loses a
/// branch: the web-login and service-account role lookups were identical apart
/// from which membership table they joined, and now share one.
async fn recreate_permission_functions(
	connection: &mut DatabaseConnection,
) -> Result<(), ErrorType> {
	sqlx::query(r#"DROP FUNCTION IF EXISTS GENERATE_LOGIN_ID();"#)
		.execute(&mut *connection)
		.await?;

	sqlx::query(
		r#"
		CREATE FUNCTION GENERATE_CREDENTIAL_ID() RETURNS UUID AS $$
		DECLARE
			id UUID;
		BEGIN
			id := gen_random_uuid();
			WHILE EXISTS(
				SELECT
					1
				FROM
					credential
				WHERE
					credential_id = id
			) LOOP
				id := gen_random_uuid();
			END LOOP;
			RETURN id;
		END;
		$$ LANGUAGE plpgsql;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(
		r#"
		CREATE OR REPLACE FUNCTION RESOURCES_WITH_PERMISSION_FOR_CREDENTIAL_ID(
			credential_id UUID,
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
			/* Workspaces where this credential has super admin access */
			WITH super_admin_workspaces AS (
				SELECT
					workspace.id AS workspace_id
				FROM
					credential
				INNER JOIN
					workspace
				ON
					workspace.super_admin_id = credential.identity_id
				WHERE
					credential.credential_id =
						RESOURCES_WITH_PERMISSION_FOR_CREDENTIAL_ID.credential_id AND
					credential.type = 'web_login'
				UNION ALL
				SELECT
					user_api_token_workspace_super_admin.workspace_id
				FROM
					user_api_token_workspace_super_admin
				WHERE
					user_api_token_workspace_super_admin.token_id =
						RESOURCES_WITH_PERMISSION_FOR_CREDENTIAL_ID.credential_id
			),
			/* Resources explicitly granted via include lists, scoped to the workspace
			the role belongs to */
			included_resources AS (
				SELECT
					role_resource_permissions_include.resource_id,
					workspace_member.workspace_id
				FROM
					credential
				INNER JOIN
					workspace_member
				ON
					workspace_member.identity_id = credential.identity_id
				INNER JOIN
					role_resource_permissions_include
				ON
					role_resource_permissions_include.role_id = workspace_member.role_id AND
					role_resource_permissions_include.permission_id = local_permission_id
				WHERE
					credential.credential_id =
						RESOURCES_WITH_PERMISSION_FOR_CREDENTIAL_ID.credential_id AND
					credential.type <> 'api_token'
				UNION ALL
				SELECT
					user_api_token_resource_permissions_include.resource_id,
					user_api_token_resource_permissions_include.workspace_id
				FROM
					user_api_token_resource_permissions_include
				WHERE
					user_api_token_resource_permissions_include.permission_id = local_permission_id AND
					user_api_token_resource_permissions_include.token_id =
						RESOURCES_WITH_PERMISSION_FOR_CREDENTIAL_ID.credential_id
			),
			/* Resources explicitly denied via exclude lists */
			excluded_resources AS (
				SELECT
					role_resource_permissions_exclude.resource_id
				FROM
					credential
				INNER JOIN
					workspace_member
				ON
					workspace_member.identity_id = credential.identity_id
				INNER JOIN
					role_resource_permissions_exclude
				ON
					role_resource_permissions_exclude.role_id = workspace_member.role_id AND
					role_resource_permissions_exclude.permission_id = local_permission_id
				WHERE
					credential.credential_id =
						RESOURCES_WITH_PERMISSION_FOR_CREDENTIAL_ID.credential_id AND
					credential.type <> 'api_token'
				UNION ALL
				SELECT
					user_api_token_resource_permissions_exclude.resource_id
				FROM
					user_api_token_resource_permissions_exclude
				WHERE
					user_api_token_resource_permissions_exclude.permission_id = local_permission_id AND
					user_api_token_resource_permissions_exclude.token_id =
						RESOURCES_WITH_PERMISSION_FOR_CREDENTIAL_ID.credential_id
			),
			/* Workspaces where this credential has any exclude-type permission */
			exclude_workspaces AS (
				SELECT
					workspace_member.workspace_id
				FROM
					credential
				INNER JOIN
					workspace_member
				ON
					workspace_member.identity_id = credential.identity_id
				INNER JOIN
					role_resource_permissions_type
				ON
					role_resource_permissions_type.role_id = workspace_member.role_id AND
					role_resource_permissions_type.permission_id = local_permission_id AND
					role_resource_permissions_type.permission_type = 'exclude'
				WHERE
					credential.credential_id =
						RESOURCES_WITH_PERMISSION_FOR_CREDENTIAL_ID.credential_id AND
					credential.type <> 'api_token'
				UNION ALL
				SELECT
					user_api_token_resource_permissions_type.workspace_id
				FROM
					user_api_token_resource_permissions_type
				WHERE
					user_api_token_resource_permissions_type.permission_id = local_permission_id AND
					user_api_token_resource_permissions_type.token_id =
						RESOURCES_WITH_PERMISSION_FOR_CREDENTIAL_ID.credential_id AND
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
						super_admin_workspaces.workspace_id = resource.owner_id
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
						included_resources.workspace_id = resource.owner_id
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
							exclude_workspaces.workspace_id = resource.owner_id
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
		"#,
	)
	.execute(&mut *connection)
	.await?;

	sqlx::query(r#"DROP FUNCTION IF EXISTS RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID(UUID, TEXT);"#)
		.execute(&mut *connection)
		.await?;

	Ok(())
}
