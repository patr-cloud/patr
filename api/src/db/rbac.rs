use models::rbac::Permission;

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
			resource_type_id UUID,
			owner_id UUID NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			deleted TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Roles belong to an workspace
	query!(
		r#"
		CREATE TABLE role(
			id UUID NOT NULL,
			name VARCHAR(100) NOT NULL,
			description VARCHAR(500) NOT NULL,
			owner_id UUID NOT NULL
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

	// Users belong to an workspace through a role
	query!(
		r#"
		CREATE TABLE workspace_user(
			user_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			role_id UUID NOT NULL
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
			ADD CONSTRAINT resource_uq_id_owner_id_deleted UNIQUE(id, owner_id, deleted);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			resource_idx_owner_id
		ON
			resource
		(owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Roles belong to an workspace
	query!(
		r#"
		ALTER TABLE role
			ADD CONSTRAINT role_pk PRIMARY KEY(id),
			ADD CONSTRAINT role_uq_name_owner_id UNIQUE(name, owner_id);
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
			ADD CONSTRAINT resource_fk_owner_id
				FOREIGN KEY(owner_id) REFERENCES workspace(id)
					DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Roles belong to an workspace
	query!(
		r#"
		ALTER TABLE role
		ADD CONSTRAINT role_fk_owner_id
		FOREIGN KEY(owner_id) REFERENCES workspace(id);
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
			ADD CONSTRAINT workspace_user_fk_role_id
				FOREIGN KEY(role_id) REFERENCES role(id);
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
	for permission in Permission::list_all_permissions() {
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
		CREATE FUNCTION GENERATE_ROLE_ID() RETURNS UUID AS $$
		DECLARE
			role_id UUID;
		BEGIN
			role_id := gen_random_uuid();
			WHILE EXISTS(
				SELECT
					1
				FROM
					role
				WHERE
					id = role_id
			) LOOP
				role_id := gen_random_uuid();
			END LOOP;
			RETURN role_id;
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
			owner_id UUID,
			created TIMESTAMPTZ,
			deleted TIMESTAMPTZ
		) AS $$
		DECLARE
			local_permission_id UUID;
		BEGIN
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

			RETURN QUERY SELECT
				resource.*
			FROM
				resource
			WHERE
				resource.owner_id IN (
					SELECT DISTINCT
						COALESCE(
							user_api_token_workspace_super_admin.workspace_id,
							workspace.id
						)
					FROM
						user_login
					LEFT JOIN
						user_api_token_workspace_super_admin
					ON
						user_login.login_type = 'api_token' AND
						user_api_token_workspace_super_admin.token_id = user_login.login_id
					LEFT JOIN
						workspace
					ON
						user_login.login_type = 'web_login' AND
						workspace.super_admin_id = user_login.user_id
					WHERE
						user_login.login_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id
				) OR resource.id IN (
					SELECT
						COALESCE(
							user_api_token_resource_permissions_include.resource_id,
							role_resource_permissions_include.resource_id
						)
					FROM
						user_login
					LEFT JOIN
						user_api_token_resource_permissions_include
					ON
						user_login.login_type = 'api_token' AND
						user_api_token_resource_permissions_include.token_id = user_login.login_id
					LEFT JOIN
						workspace_user
					ON
						workspace_user.user_id = user_login.user_id
					LEFT JOIN
						role_resource_permissions_include
					ON
						role_resource_permissions_include.role_id = workspace_user.role_id
					WHERE
						user_login.login_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id AND
						role_resource_permissions_include.permission_id = local_permission_id AND
						resource.owner_id = COALESCE(
							user_api_token_resource_permissions_include.workspace_id,
							workspace_user.workspace_id
						)
				) OR resource.id NOT IN (
					SELECT
						COALESCE(
							user_api_token_resource_permissions_exclude.resource_id,
							role_resource_permissions_exclude.resource_id
						)
					FROM
						user_login
					LEFT JOIN
						user_api_token_resource_permissions_exclude
					ON
						user_login.login_type = 'api_token' AND
						user_api_token_resource_permissions_exclude.token_id = user_login.login_id
					LEFT JOIN
						workspace_user
					ON
						workspace_user.user_id = user_login.user_id
					LEFT JOIN
						role_resource_permissions_exclude
					ON
						role_resource_permissions_exclude.role_id = workspace_user.role_id
					WHERE
						user_login.login_id = RESOURCES_WITH_PERMISSION_FOR_LOGIN_ID.login_id AND
						role_resource_permissions_exclude.permission_id = local_permission_id AND
						resource.owner_id = COALESCE(
							user_api_token_resource_permissions_exclude.workspace_id,
							workspace_user.workspace_id
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
