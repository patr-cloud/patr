use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	add_rbac_blocklist_tables(connection, config).await?;
	Ok(())
}

async fn add_rbac_blocklist_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// Add RBAC tables for role
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
		CREATE TABLE role_resource_permissions_type(
			role_id 		UUID 			NOT NULL,
			permission_id 	UUID 			NOT NULL,
			permission_type PERMISSION_TYPE NOT NULL,
			CONSTRAINT role_resource_permissions_type_pk PRIMARY KEY(
				role_id,
				permission_id
			),
			CONSTRAINT role_resource_permissions_type_uq UNIQUE(
				role_id,
				permission_id,
				permission_type
			),
			CONSTRAINT role_resource_permissions_type_fk_role_id
				FOREIGN KEY(role_id) REFERENCES role(id),
			CONSTRAINT role_resource_permissions_type_fk_permission_id
				FOREIGN KEY(permission_id) REFERENCES permission(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE role_resource_permissions_include(
			role_id 		UUID 			NOT NULL,
			permission_id 	UUID 			NOT NULL,
			resource_id		UUID			NOT NULL,
			permission_type PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS('include') STORED,

			CONSTRAINT role_resource_permissions_include_pk PRIMARY KEY(
				role_id,
				permission_id,
				resource_id
			),
			CONSTRAINT role_resource_permissions_include_fk_parent FOREIGN KEY(
				role_id,
				permission_id,
				permission_type
			) REFERENCES role_resource_permissions_type(
				role_id,
				permission_id,
				permission_type
			),
			CONSTRAINT role_resource_permissions_include_fk_resource
				FOREIGN KEY(resource_id) REFERENCES resource(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE role_resource_permissions_exclude(
			role_id 		UUID 			NOT NULL,
			permission_id 	UUID 			NOT NULL,
			resource_id		UUID			NOT NULL,
			permission_type PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('exclude') STORED,

			CONSTRAINT role_resource_permissions_exclude_pk PRIMARY KEY(
				role_id,
				permission_id,
				resource_id
			),
			CONSTRAINT role_resource_permissions_exclude_fk_parent
				FOREIGN KEY(role_id, permission_id, permission_type)
					REFERENCES role_resource_permissions_type(
						role_id,
						permission_id,
						permission_type
					),
			CONSTRAINT role_resource_permissions_exclude_fk_resource
				FOREIGN KEY(resource_id) REFERENCES resource(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			role_resource_permissions_type(
				role_id,
				permission_id,
				permission_type
			)
		SELECT
			role_id,
			permission_id,
			'exclude'
		FROM
			role_permissions_resource_type
		ON CONFLICT DO NOTHING;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			role_resource_permissions_type(
				role_id,
				permission_id,
				permission_type
			)
		SELECT
			role_id,
			permission_id,
			'include'
		FROM
			role_permissions_resource
		ON CONFLICT DO NOTHING;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			role_resource_permissions_include(
				role_id,
				permission_id,
				resource_id
			)
		SELECT
			role_id,
			permission_id,
			resource_id
		FROM
			role_permissions_resource;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE role_permissions_resource;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE role_permissions_resource_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Add RBAC tables for user API token
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
		CREATE TABLE user_api_token_workspace_permission_type(
			token_id 				UUID 					NOT NULL,
			workspace_id			UUID 					NOT NULL,
			token_permission_type 	TOKEN_PERMISSION_TYPE 	NOT NULL,
			CONSTRAINT user_api_token_workspace_permission_type_pk PRIMARY KEY(
				token_id,
				workspace_id
			),
			CONSTRAINT user_api_token_workspace_permission_type_uq UNIQUE(
				token_id,
				workspace_id,
				token_permission_type
			),
			CONSTRAINT user_api_token_workspace_permission_type_fk_token
				FOREIGN KEY(token_id) REFERENCES user_api_token(token_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_workspace_super_admin
		ADD COLUMN token_permission_type TOKEN_PERMISSION_TYPE NOT NULL
		GENERATED ALWAYS AS ('super_admin') STORED;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_workspace_super_admin
			DROP CONSTRAINT user_api_token_workspace_super_admin_pk,
			ADD CONSTRAINT user_api_token_workspace_super_admin_pk PRIMARY KEY(
				token_id, user_id, workspace_id
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			user_api_token_workspace_permission_type
		SELECT
			token_id,
			workspace_id,
			'super_admin'
		FROM
			user_api_token_workspace_super_admin;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			user_api_token_workspace_permission_type
		SELECT
			user_api_token.token_id,
			COALESCE(
				user_api_token_resource_type_permission.workspace_id,
				user_api_token_resource_permission.workspace_id
			),
			'member'
		FROM
			user_api_token
		LEFT JOIN
			user_api_token_resource_permission
		ON
			user_api_token_resource_permission.token_id = user_api_token.token_id
		LEFT JOIN
			user_api_token_resource_type_permission
		ON
			user_api_token_resource_type_permission.token_id = user_api_token.token_id
		WHERE
			user_api_token.token_id NOT IN (
				SELECT
					token_id
				FROM
					user_api_token_workspace_super_admin
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_api_token_workspace_super_admin
		ADD CONSTRAINT user_api_token_workspace_super_admin_fk_type
		FOREIGN KEY(token_id, workspace_id, token_permission_type)
		REFERENCES user_api_token_workspace_permission_type(
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
		CREATE TABLE user_api_token_resource_permissions_type(
			token_id 					UUID 			NOT NULL,
			workspace_id 				UUID 			NOT NULL,
			permission_id 				UUID 			NOT NULL,
			resource_permission_type	PERMISSION_TYPE NOT NULL,
			token_permission_type TOKEN_PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('member') STORED,
			CONSTRAINT user_api_token_resource_permissions_type_pk PRIMARY KEY(
				token_id,
				workspace_id,
				permission_id
			),
			CONSTRAINT user_api_token_resource_permissions_type_uq UNIQUE(
				token_id,
				workspace_id,
				permission_id,
				resource_permission_type
			),
			CONSTRAINT user_api_token_resource_permisssions_type_fk_type
				FOREIGN KEY(token_id, workspace_id, token_permission_type)
					REFERENCES user_api_token_workspace_permission_type(
						token_id,
						workspace_id,
						token_permission_type
					),
			CONSTRAINT
				user_api_token_resource_permisssions_type_fk_permission_id
					FOREIGN KEY(permission_id) REFERENCES permission(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_resource_permissions_include(
			token_id 		UUID 			NOT NULL,
			workspace_id 	UUID 			NOT NULL,
			permission_id 	UUID 			NOT NULL,
			resource_id		UUID			NOT NULL,
			permission_type PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('include') STORED,
			CONSTRAINT user_api_token_resource_permissions_include_pk
				PRIMARY KEY(token_id, workspace_id, permission_id, resource_id),
			CONSTRAINT user_api_token_resource_permissions_include_fk_parent
				FOREIGN KEY(
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
			CONSTRAINT user_api_token_resource_permissions_include_fk_resource
				FOREIGN KEY(resource_id, workspace_id) REFERENCES resource(
					id,
					owner_id
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_resource_permissions_exclude(
			token_id 		UUID 			NOT NULL,
			workspace_id 	UUID 			NOT NULL,
			permission_id 	UUID 			NOT NULL,
			resource_id		UUID			NOT NULL,
			permission_type PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('exclude') STORED,
			CONSTRAINT user_api_token_resource_permissions_exclude_pk
				PRIMARY KEY(token_id, workspace_id, permission_id, resource_id),
			CONSTRAINT user_api_token_resource_permissions_exclude_fk_parent
				FOREIGN KEY(
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
			CONSTRAINT user_api_token_resource_permissions_exclude_fk_resource
				FOREIGN KEY(resource_id, workspace_id) REFERENCES resource(
					id,
					owner_id
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			user_api_token_resource_permissions_type(
				token_id,
				workspace_id,
				permission_id,
				resource_permission_type
			)
		SELECT
			token_id,
			workspace_id,
			permission_id,
			'exclude'
		FROM
			user_api_token_resource_type_permission
		ON CONFLICT DO NOTHING;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			user_api_token_resource_permissions_type(
				token_id,
				workspace_id,
				permission_id,
				resource_permission_type
			)
		SELECT
			token_id,
			workspace_id,
			permission_id,
			'include'
		FROM
			user_api_token_resource_permission
		ON CONFLICT DO NOTHING;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			user_api_token_resource_permissions_include(
				token_id,
				workspace_id,
				permission_id,
				resource_id
			)
		SELECT
			token_id,
			workspace_id,
			permission_id,
			resource_id
		FROM
			user_api_token_resource_permission;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE user_api_token_resource_permission;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TABLE user_api_token_resource_type_permission;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
