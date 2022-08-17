use std::{collections::BTreeMap, net::IpAddr};

use api_models::{
	models::user::{BasicUserInfo, UserPhoneNumber},
	utils::{ResourceType, Uuid},
};
use chrono::{DateTime, Utc};

use crate::{db::Workspace, query, query_as, Database};

pub struct ApiToken {
	pub token: Uuid,
	pub user_id: Uuid,
	pub token_expiry: Option<DateTime<Utc>>,
	pub created: DateTime<Utc>,
	pub revoked: bool,
}

pub async fn initialize_api_token_pre(
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
			revoked_by UUID
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE api_token_resource_permission(
			token UUID NOT NULL,
			workspace_id UUID NOT NULL,
			permission_id UUID NOT NULL,
			resource_id UUID NOT NULL,
			CONSTRAINT api_token_resource_permission_fk_token
				FOREIGN KEY(token)
					REFERENCES api_token(token)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE api_token_resource_type_permission(
			token UUID NOT NULL,
			workspace_id UUID NOT NULL,
			permission_id UUID NOT NULL,
			resource_type_id UUID NOT NULL,
			CONSTRAINT api_token_resource_permission_type_fk_token
				FOREIGN KEY(token)
					REFERENCES api_token(token)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE api_token_workspace_super_admin(
			token UUID NOT NULL,
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

pub async fn initialize_api_token_post(
	_connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	Ok(())
}

pub async fn revoke_user_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token: &Uuid,
	revoked_by: &Uuid,
	revoked: bool,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			api_token
		SET
			revoked = $1,
			revoked_by = $2
		WHERE
			token = $3;
		"#,
		revoked,
		revoked_by as _,
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
			token_expiry as "token_expiry!: _",
			created as "created: _",
			revoked
		FROM
			api_token
		WHERE
			token = $1;
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
