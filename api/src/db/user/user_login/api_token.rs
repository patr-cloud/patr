use crate::prelude::*;

/// Initializes the API token tables
#[instrument(skip(connection))]
pub async fn initialize_api_token_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up API token tables");
	query!(
		r#"
		CREATE TYPE TOKEN_PERMISSION_TYPE AS ENUM(
			'super_admin',
			'member'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE PERMISSION_TYPE AS ENUM(
			'include',
			'exclude'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token(
			token_id UUID NOT NULL,
			name TEXT NOT NULL,
			user_id UUID NOT NULL,
			token_hash TEXT NOT NULL,
			token_nbf TIMESTAMPTZ, /* The token is not valid before this date */
			token_exp TIMESTAMPTZ, /* The token is not valid after this date */
			allowed_ips INET[],
			created TIMESTAMPTZ NOT NULL,
			revoked TIMESTAMPTZ,
			login_type USER_LOGIN_TYPE GENERATED ALWAYS AS ('api_token') STORED
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_workspace_permission_type(
			token_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			token_permission_type TOKEN_PERMISSION_TYPE NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_workspace_super_admin(
			token_id UUID NOT NULL,
			user_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			token_permission_type TOKEN_PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('super_admin') STORED
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_resource_permissions_type(
			token_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			permission_id UUID NOT NULL,
			resource_permission_type PERMISSION_TYPE NOT NULL,
			token_permission_type TOKEN_PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('member') STORED
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_resource_permissions_include(
			token_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			permission_id UUID NOT NULL,
			resource_id UUID NOT NULL,
			resource_deleted TIMESTAMPTZ GENERATED ALWAYS AS (NULL) STORED,
			permission_type PERMISSION_TYPE NOT NULL GENERATED ALWAYS AS ('include') STORED
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_resource_permissions_exclude(
			token_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			permission_id UUID NOT NULL,
			resource_id UUID NOT NULL,
			resource_deleted TIMESTAMPTZ GENERATED ALWAYS AS (NULL) STORED,
			permission_type PERMISSION_TYPE NOT NULL GENERATED ALWAYS AS ('exclude') STORED
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the API token indices
#[instrument(skip(connection))]
pub async fn initialize_api_token_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up API token indices");
	query!(
		r#"
		ALTER TABLE user_api_token
			ADD CONSTRAINT user_api_token_pk PRIMARY KEY(token_id),
			ADD CONSTRAINT user_api_token_token_id_user_id_uk UNIQUE(
				token_id, user_id
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			user_api_token_uq_name_user_id
		ON
			user_api_token(name, user_id)
		WHERE
			revoked IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_workspace_permission_type
			ADD CONSTRAINT user_api_token_workspace_permission_type_pk PRIMARY KEY(
				token_id,
				workspace_id
			),
			ADD CONSTRAINT user_api_token_workspace_permission_type_uq UNIQUE(
				token_id,
				workspace_id,
				token_permission_type
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_workspace_super_admin
		ADD CONSTRAINT user_api_token_workspace_super_admin_pk
		PRIMARY KEY(token_id, user_id, workspace_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_resource_permissions_type
			ADD CONSTRAINT user_api_token_resource_permissions_type_pk PRIMARY KEY(
				token_id,
				workspace_id,
				permission_id
			),
			ADD CONSTRAINT user_api_token_resource_permissions_type_uq UNIQUE(
				token_id,
				workspace_id,
				permission_id,
				resource_permission_type
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_resource_permissions_include
		ADD CONSTRAINT user_api_token_resource_permissions_include_pk
		PRIMARY KEY(token_id, workspace_id, permission_id, resource_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_resource_permissions_exclude
		ADD CONSTRAINT user_api_token_resource_permissions_exclude_pk
		PRIMARY KEY(token_id, workspace_id, permission_id, resource_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

/// Initializes the API token constraints
#[instrument(skip(connection))]
pub async fn initialize_api_token_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up API token constraints");
	query!(
		r#"
		ALTER TABLE user_api_token
		ADD CONSTRAINT user_api_token_token_id_user_id_login_type_fk
		FOREIGN KEY(token_id, user_id, login_type) REFERENCES user_login(
			login_id, user_id, login_type
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_workspace_permission_type
		ADD CONSTRAINT user_api_token_workspace_permission_type_fk_token
		FOREIGN KEY(token_id) REFERENCES user_api_token(token_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_workspace_super_admin
			ADD CONSTRAINT user_api_token_workspace_super_admin_fk_type FOREIGN KEY(
				token_id,
				workspace_id,
				token_permission_type
			) REFERENCES user_api_token_workspace_permission_type(
				token_id,
				workspace_id,
				token_permission_type
			),
			ADD CONSTRAINT user_api_token_workspace_super_admin_fk_token FOREIGN KEY(
				token_id,
				user_id
			) REFERENCES user_api_token(
				token_id,
				user_id
			),
			ADD CONSTRAINT user_api_token_workspace_super_admin_fk_workspace FOREIGN KEY(
				workspace_id,
				user_id
			) REFERENCES workspace(
				id,
				super_admin_id
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_resource_permissions_type
			ADD CONSTRAINT user_api_token_resource_permissions_type_fk_type FOREIGN KEY(
				token_id,
				workspace_id,
				token_permission_type
			) REFERENCES user_api_token_workspace_permission_type(
				token_id,
				workspace_id,
				token_permission_type
			),
			ADD CONSTRAINT user_api_token_resource_permissions_type_fk_permission_id FOREIGN KEY(
				permission_id
			) REFERENCES permission(
				id
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_resource_permissions_include
			ADD CONSTRAINT user_api_token_resource_permissions_include_fk_parent FOREIGN KEY(
				token_id,
				workspace_id,
				permission_id,
				permission_type
			) REFERENCES user_api_token_resource_permissions_type(
				token_id,
				workspace_id,
				permission_id,
				resource_permission_type
			),
			ADD CONSTRAINT user_api_token_resource_permissions_include_fk_resource FOREIGN KEY(
				resource_id,
				workspace_id,
				resource_deleted
			) REFERENCES resource(
				id,
				owner_id,
				deleted
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_resource_permissions_exclude
			ADD CONSTRAINT user_api_token_resource_permissions_exclude_fk_parent FOREIGN KEY(
				token_id,
				workspace_id,
				permission_id,
				permission_type
			) REFERENCES user_api_token_resource_permissions_type(
				token_id,
				workspace_id,
				permission_id,
				resource_permission_type
			),
			ADD CONSTRAINT user_api_token_resource_permissions_exclude_fk_resource FOREIGN KEY(
				resource_id,
				workspace_id,
				resource_deleted
			) REFERENCES resource(
				id,
				owner_id,
				deleted
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
