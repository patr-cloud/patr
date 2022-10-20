use std::collections::{BTreeMap, HashMap, HashSet};

use api_models::utils::Uuid;
use chrono::{DateTime, Utc};
use sqlx::types::ipnetwork::IpNetwork;

use super::UserLoginType;
use crate::{models::rbac::WorkspacePermissions, query, query_as, Database};

pub struct UserApiToken {
	pub token_id: Uuid,
	pub name: String,
	pub user_id: Uuid,
	pub token_hash: String,
	pub token_nbf: Option<DateTime<Utc>>,
	pub token_exp: Option<DateTime<Utc>>,
	pub allowed_ips: Option<Vec<IpNetwork>>,
	pub revoked: Option<DateTime<Utc>>,
	pub created: DateTime<Utc>,
}

pub async fn initialize_api_token_pre(
	_connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	// tables are initialized in post due to workspace dependency
	Ok(())
}

pub async fn initialize_api_token_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE user_api_token(
			token_id UUID
				CONSTRAINT user_api_token_pk PRIMARY KEY,
			name TEXT NOT NULL,
			user_id UUID NOT NULL,
			token_hash TEXT NOT NULL,
			token_nbf TIMESTAMPTZ, /* The token is not valid before this date */
			token_exp TIMESTAMPTZ, /* The token is not valid after this date */
			allowed_ips INET[],
			created TIMESTAMPTZ NOT NULL DEFAULT NOW(),
			revoked TIMESTAMPTZ,
			login_type USER_LOGIN_TYPE NOT NULL
				GENERATED ALWAYS AS ('api_token_login') STORED,
			CONSTRAINT user_api_token_token_id_user_id_uk
				UNIQUE (token_id, user_id),
			CONSTRAINT user_api_token_token_id_user_id_login_type_fk
				FOREIGN KEY(token_id, user_id, login_type)
					REFERENCES user_login(login_id, user_id, login_type)
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
		CREATE TABLE user_api_token_workspace_super_admin(
			token_id UUID NOT NULL,
			user_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			CONSTRAINT user_api_token_workspace_super_admin_fk_token
				FOREIGN KEY(token_id, user_id)
					REFERENCES user_api_token(token_id, user_id),
			CONSTRAINT user_api_token_workspace_super_admin_fk_workspace
				FOREIGN KEY(workspace_id, user_id)
					REFERENCES workspace(id, super_admin_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_resource_permission(
			token_id UUID NOT NULL
				CONSTRAINT user_api_token_resource_permission_fk_token_id
					REFERENCES user_api_token(token_id),
			workspace_id UUID NOT NULL,
			resource_id UUID NOT NULL,
			permission_id UUID NOT NULL
				CONSTRAINT user_api_token_resource_permission_fk_permission_id
					REFERENCES permission(id),
			CONSTRAINT user_api_token_resource_permission_workspace_id_resource_id
				FOREIGN KEY (workspace_id, resource_id)
					REFERENCES resource(owner_id, id),
			CONSTRAINT user_api_token_resource_permission_pk 
				PRIMARY KEY(token_id, permission_id, resource_id, workspace_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_resource_type_permission(
			token_id UUID NOT NULL
				CONSTRAINT user_api_token_resource_type_permission_fk_token_id
					REFERENCES user_api_token(token_id),
			workspace_id UUID NOT NULL
				CONSTRAINT user_api_token_resource_type_permission_fk_workspace_id
					REFERENCES workspace(id),
			resource_type_id UUID NOT NULL
				CONSTRAINT user_api_token_resource_type_permission_fk_resource_type_id
					REFERENCES resource_type(id),
			permission_id UUID NOT NULL
				CONSTRAINT user_api_token_resource_type_permission_fk_permission_id
					REFERENCES permission(id),
			CONSTRAINT user_api_token_resource_type_permission_pk
				PRIMARY KEY(token_id, permission_id, resource_type_id, workspace_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn revoke_user_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user_api_token
		SET
			revoked = NOW()
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_currently_active_api_token_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
) -> Result<Option<UserApiToken>, sqlx::Error> {
	query_as!(
		UserApiToken,
		r#"
		SELECT
			token_id as "token_id: _",
			name,
			user_id as "user_id: _",
			token_hash,
			token_nbf,
			token_exp,
			allowed_ips,
			created,
			revoked
		FROM
			user_api_token
		WHERE
			token_id = $1 AND
			revoked IS NULL AND
			(token_exp IS NULL OR token_exp > NOW()) AND
			(token_nbf IS NULL OR token_nbf < NOW());
		"#,
		token_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn create_api_token_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
	name: &str,
	user_id: &Uuid,
	token_hash: &str,
	token_nbf: Option<&DateTime<Utc>>,
	token_exp: Option<&DateTime<Utc>>,
	allowed_ips: Option<&[IpNetwork]>,
) -> Result<(), sqlx::Error> {
	super::add_new_user_login(
		connection,
		token_id,
		user_id,
		&UserLoginType::ApiTokenLogin,
	)
	.await?;

	query!(
		r#"
		INSERT INTO
			user_api_token (
				token_id,
				name,
				user_id,
				token_hash,
				token_nbf,
				token_exp,
				allowed_ips
			)
		VALUES
			($1, $2, $3, $4, $5, $6, $7);
		"#,
		token_id as _,
		name,
		user_id as _,
		token_hash,
		token_nbf,
		token_exp,
		allowed_ips
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_super_admin_info_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
	workspace_id: &Uuid,
	user_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_api_token_workspace_super_admin(
				token_id,
				user_id,
				workspace_id
			)
		VALUES
			($1, $2, $3);
		"#,
		token_id as _,
		user_id as _,
		workspace_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_resource_permission_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
	workspace_id: &Uuid,
	resource_permissions: &BTreeMap<Uuid, Vec<Uuid>>,
) -> Result<(), sqlx::Error> {
	for (resource_id, permissions) in resource_permissions {
		for permission_id in permissions {
			query!(
				r#"
				INSERT INTO
					user_api_token_resource_permission (
						token_id,
						workspace_id,
						resource_id,
						permission_id
					)
				VALUES
					($1, $2, $3, $4);
				"#,
				token_id as _,
				workspace_id as _,
				resource_id as _,
				permission_id as _
			)
			.execute(&mut *connection)
			.await?;
		}
	}
	Ok(())
}

pub async fn add_resource_type_permission_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
	workspace_id: &Uuid,
	resource_type_permissions: &BTreeMap<Uuid, Vec<Uuid>>,
) -> Result<(), sqlx::Error> {
	for (resource_type_id, permissions) in resource_type_permissions {
		for permission_id in permissions {
			query!(
				r#"
				INSERT INTO
					user_api_token_resource_type_permission(
						token_id,
						workspace_id,
						resource_type_id,
						permission_id
					)
				VALUES
					($1, $2, $3, $4);	
				"#,
				token_id as _,
				workspace_id as _,
				resource_type_id as _,
				permission_id as _
			)
			.execute(&mut *connection)
			.await?;
		}
	}
	Ok(())
}

pub async fn get_user_permissions_resource_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	workspace_id: &Uuid,
) -> Result<BTreeMap<Uuid, HashSet<Uuid>>, sqlx::Error> {
	let mut permissions = BTreeMap::<Uuid, HashSet<Uuid>>::new();
	let rows = query!(
		r#"
		SELECT
			resource_id as "resource_id: Uuid",
			permission_id as "permission_id: Uuid" 
		FROM
			role_permissions_resource
		LEFT JOIN
			workspace_user 
		ON 
			workspace_user.role_id = role_permissions_resource.role_id
		WHERE
			workspace_user.workspace_id = $1 AND
			workspace_user.user_id = $2;
		"#,
		workspace_id as _,
		user_id as _,
	)
	.fetch_all(&mut *connection)
	.await?;

	for row in rows {
		let permission_id = row.permission_id;
		permissions
			.entry(row.resource_id)
			.or_insert_with(HashSet::new)
			.insert(permission_id);
	}

	Ok(permissions)
}

pub async fn get_user_permissions_resource_type_in_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	workspace_id: &Uuid,
) -> Result<BTreeMap<Uuid, HashSet<Uuid>>, sqlx::Error> {
	let mut permissions = BTreeMap::<Uuid, HashSet<Uuid>>::new();

	let rows = query!(
		r#"
		SELECT
			resource_type_id as "resource_type_id: Uuid",
			permission_id as "permission_id: Uuid"
		FROM
			role_permissions_resource_type
		LEFT JOIN
			workspace_user 
		ON 
			workspace_user.role_id = role_permissions_resource_type.role_id
		WHERE
			workspace_user.workspace_id = $1 AND
			workspace_user.user_id = $2;
		"#,
		workspace_id as _,
		user_id as _,
	)
	.fetch_all(&mut *connection)
	.await?;

	for row in rows {
		let permission_id = row.permission_id;
		permissions
			.entry(row.resource_type_id)
			.or_insert_with(HashSet::new)
			.insert(permission_id);
	}

	Ok(permissions)
}

pub async fn is_user_super_admin(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	user_id: &Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
	let record = query!(
		r#"
		SELECT
			super_admin_id AS "super_admin_id: Uuid"
		FROM
			workspace
		WHERE
			super_admin_id = $1 AND
			id = $2;
		"#,
		user_id as _,
		workspace_id as _,
	)
	.fetch_optional(&mut *connection)
	.await?
	.map(|row| row.super_admin_id);

	Ok(record)
}

pub async fn list_currently_active_api_tokens_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Vec<UserApiToken>, sqlx::Error> {
	query_as!(
		UserApiToken,
		r#"
		SELECT
			token_id as "token_id: _",
			name,
			user_id as "user_id: _",
			token_hash,
			token_nbf,
			token_exp,
			allowed_ips,
			created,
			revoked
		FROM
			user_api_token
		WHERE
			user_id = $1 AND
			revoked IS NULL AND
			(token_exp IS NULL OR token_exp > NOW()) AND
			(token_nbf IS NULL OR token_nbf < NOW());
		"#,
		user_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_permissions_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
) -> Result<HashMap<Uuid, WorkspacePermissions>, sqlx::Error> {
	let mut workspace_permissions = HashMap::new();

	// super admin permissions
	query!(
		r#"
		SELECT
			workspace_id as "workspace_id: Uuid"
		FROM
			user_api_token_workspace_super_admin
		WHERE
			token_id = $1;
		"#,
		token_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|record| record.workspace_id)
	.for_each(|workspace_id: Uuid| {
		let workspace_permission: &mut WorkspacePermissions =
			workspace_permissions.entry(workspace_id).or_default();
		workspace_permission.is_super_admin = true;
	});

	// resource permission
	query!(
		r#"
		SELECT
			workspace_id as "workspace_id: Uuid",
			resource_id as "resource_id: Uuid",
			ARRAY_AGG(permission_id) as "permissions!: Vec<Uuid>"
		FROM
			user_api_token_resource_permission
		WHERE
			token_id = $1
		GROUP BY
			(workspace_id, resource_id);
		"#,
		token_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|record| (record.workspace_id, record.resource_id, record.permissions))
	.for_each(
		|(workspace_id, resource_id, permissions): (Uuid, Uuid, Vec<Uuid>)| {
			let workspace_permission: &mut WorkspacePermissions =
				workspace_permissions.entry(workspace_id).or_default();
			workspace_permission
				.resources
				.insert(resource_id, permissions);
		},
	);

	// resource type permission
	query!(
		r#"
		SELECT
			workspace_id as "workspace_id: Uuid",
			resource_type_id as "resource_type_id: Uuid",
			ARRAY_AGG(permission_id) as "permissions!: Vec<Uuid>"
		FROM
			user_api_token_resource_type_permission
		WHERE
			token_id = $1
		GROUP BY
			(workspace_id, resource_type_id);
		"#,
		token_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|record| {
		(
			record.workspace_id,
			record.resource_type_id,
			record.permissions,
		)
	})
	.for_each(
		|(workspace_id, resource_type_id, permissions): (
			Uuid,
			Uuid,
			Vec<Uuid>,
		)| {
			let workspace_permission: &mut WorkspacePermissions =
				workspace_permissions.entry(workspace_id).or_default();
			workspace_permission
				.resource_types
				.insert(resource_type_id, permissions);
		},
	);

	Ok(workspace_permissions)
}
