use std::collections::{BTreeMap, HashMap, HashSet};

use api_models::utils::Uuid;
use chrono::{DateTime, Utc};

use crate::{query, query_as, Database};

pub struct ApiToken {
	pub token: Uuid,
	pub user_id: Uuid,
	pub name: String,
	pub token_expiry: Option<DateTime<Utc>>,
	pub created: DateTime<Utc>,
}

pub struct SuperAdminApiToken {
	pub token: Uuid,
	pub workspace_id: Uuid,
	pub super_admin_id: Uuid,
}

pub struct Permission {
	pub resource_permissions: HashMap<Uuid, Vec<Uuid>>,
	pub resource_type_permissions: HashMap<Uuid, Vec<Uuid>>,
}

pub async fn initialize_api_token_pre(
	_connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	Ok(())
}

pub async fn initialize_api_token_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE api_token(
			token UUID
				CONSTRAINT api_token_pk PRIMARY KEY,
			user_id UUID NOT NULL,
			name TEXT NOT NULL,
			token_expiry TIMESTAMPTZ,
			created TIMESTAMPTZ NOT NULL,
			revoked BOOLEAN NOT NULL DEFAULT FALSE,
			revoked_by UUID,
            CONSTRAINT api_token_chk_revoked_revoked_by_valid
                CHECK(
                    (
                        revoked IS false AND
                        revoked_by IS NULL
                    ) OR (
                        revoked IS true AND
                        revoked_by IS NOT NULL
                    )
                )
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE api_token_resource_permission(
			token UUID NOT NULL
				CONSTRAINT api_token_resource_permission_fk_token
					REFERENCES api_token(token),
			workspace_id UUID NOT NULL
				CONSTRAINT api_token_resource_permission_fk_workspace_id
					REFERENCES workspace(id),
			permission_id UUID NOT NULL
				CONSTRAINT api_token_resource_permission_fk_permission_id
					REFERENCES permission(id),
			resource_id UUID NOT NULL
				CONSTRAINT api_token_resource_permission_fk_resource_id
					REFERENCES resource(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE api_token_resource_type_permission(
			token UUID NOT NULL
				CONSTRAINT api_token_resource_permission_type_fk_token
					REFERENCES api_token(token),
			workspace_id UUID NOT NULL
				CONSTRAINT api_token_resource_type_permission_fk_workspace_id
					REFERENCES workspace(id),
			permission_id UUID NOT NULL
				CONSTRAINT api_token_resource_type_permission_fk_permission_id
					REFERENCES permission (id),
			resource_type_id UUID NOT NULL
				CONSTRAINT api_token_resource_type_permission_fk_resource_type_id
					REFERENCES resource_type(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE api_token_workspace_super_admin(
			token UUID NOT NULL
				CONSTRAINT api_token_workspace_super_admin_fk_token
					REFERENCES api_token(token),
			workspace_id UUID NOT NULL,
			super_admin_id UUID NOT NULL,
			CONSTRAINT api_token_fk_workspace_id_super_admin_id
				FOREIGN KEY(workspace_id, super_admin_id)
					REFERENCES workspace(id, super_admin_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn revoke_user_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token: &Uuid,
	revoked_by: &Uuid,
	revoked: bool,
	name: String,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			api_token
		SET
			revoked = $1,
			revoked_by = $2,
			name = $3
		WHERE
			token = $4;
		"#,
		revoked,
		revoked_by as _,
		name,
		token as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_api_token_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	token: &Uuid,
) -> Result<Option<ApiToken>, sqlx::Error> {
	query_as!(
		ApiToken,
		r#"
		SELECT
			token as "token: _",
			user_id as "user_id: _",
			name,
			token_expiry as "token_expiry!: _",
			created as "created: _"
		FROM
			api_token
		WHERE
			token = $1 AND
            revoked = false;
		"#,
		token as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn create_api_token_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	token: &Uuid,
	user_id: &Uuid,
	name: &str,
	ttl: Option<DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			api_token(
				token,
				name,
				user_id,
				token_expiry,
				created,
				revoked,
				revoked_by
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				now(),
				false,
				NULL
			);
		"#,
		token as _,
		name,
		user_id as _,
		ttl as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_super_admin_info_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token: &Uuid,
	workspace_id: &Uuid,
	super_admin_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			api_token_workspace_super_admin(
				token,
				workspace_id,
				super_admin_id
			)
		VALUES
			(
				$1,
				$2,
				$3
			);
		"#,
		token as _,
		workspace_id as _,
		super_admin_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_resource_permission_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token: &Uuid,
	workspace_id: &Uuid,
	resource_permissions: &BTreeMap<Uuid, Vec<Uuid>>,
) -> Result<(), sqlx::Error> {
	for (resource_id, permissions) in resource_permissions {
		for permission_id in permissions {
			query!(
				r#"
				INSERT INTO
					api_token_resource_permission(
						token,
						workspace_id,
						resource_id,
						permission_id
					)
				VALUES
					($1, $2, $3, $4);
				"#,
				token as _,
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
	token: &Uuid,
	workspace_id: &Uuid,
	resource_permissions: &BTreeMap<Uuid, Vec<Uuid>>,
) -> Result<(), sqlx::Error> {
	for (resource_type_id, permissions) in resource_permissions {
		for permission_id in permissions {
			query!(
				r#"
				INSERT INTO
					api_token_resource_type_permission(
						token,
						workspace_id,
						resource_type_id,
						permission_id
					)
				VALUES
					($1, $2, $3, $4);	
				"#,
				token as _,
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

pub async fn list_api_tokens_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Vec<ApiToken>, sqlx::Error> {
	query_as!(
		ApiToken,
		r#"
		SELECT
			token as "token: _",
			name,
			user_id as "user_id: _",
			token_expiry as "token_expiry!: _",
			created as "created: _"
		FROM
			api_token
		WHERE
			user_id = $1 AND
			revoked = false;
		"#,
		user_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn list_permissions_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token: &Uuid,
) -> Result<Permission, sqlx::Error> {
	let mut resource_permissions = HashMap::<Uuid, Vec<Uuid>>::new();
	let mut resource_type_permissions = HashMap::<Uuid, Vec<Uuid>>::new();

	let rows = query!(
		r#"
		SELECT
			resource_id as "resource_id: Uuid",
			permission_id as "permission_id: Uuid"
		FROM
			api_token_resource_permission
		WHERE
			token = $1;
		"#,
		token as _,
	)
	.fetch_all(&mut *connection)
	.await?;

	for row in rows {
		resource_permissions
			.entry(row.resource_id)
			.or_insert_with(Vec::new)
			.push(row.permission_id);
	}

	let rows = query!(
		r#"
		SELECT
			resource_type_id as "resource_type_id: Uuid",
			permission_id as "permission_id: Uuid"
		FROM
			api_token_resource_type_permission
		WHERE
			token = $1;
		"#,
		token as _,
	)
	.fetch_all(&mut *connection)
	.await?;

	for row in rows {
		resource_type_permissions
			.entry(row.resource_type_id)
			.or_insert_with(Vec::new)
			.push(row.permission_id);
	}

	Ok(Permission {
		resource_permissions,
		resource_type_permissions,
	})
}

pub async fn get_workspace_id_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	api_token: &Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
	let resource_permission_workspace_id = query!(
		r#"
		SELECT
			workspace_id as "workspace_id: Uuid"
		FROM
			api_token_resource_permission
		WHERE
			token = $1;
		"#,
		api_token as _
	)
	.fetch_optional(&mut *connection)
	.await?;

	let resource_type_permission_workspace_id = query!(
		r#"
		SELECT
			workspace_id as "workspace_id: Uuid"
		FROM
			api_token_resource_type_permission
		WHERE
			token = $1;
		"#,
		api_token as _
	)
	.fetch_optional(&mut *connection)
	.await?;

	if let Some(record) = resource_permission_workspace_id {
		Ok(Some(record.workspace_id))
	} else if let Some(record) = resource_type_permission_workspace_id {
		Ok(Some(record.workspace_id))
	} else {
		Ok(None)
	}
}

pub async fn get_super_admin_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	api_token: &Uuid,
) -> Result<Option<SuperAdminApiToken>, sqlx::Error> {
	query_as!(
		SuperAdminApiToken,
		r#"
		SELECT
			token as "token: _",
			workspace_id as "workspace_id: _",
			super_admin_id as "super_admin_id: _"
		FROM
			api_token_workspace_super_admin
		WHERE
			token = $1;
		"#,
		api_token as _
	)
	.fetch_optional(&mut *connection)
	.await
}
