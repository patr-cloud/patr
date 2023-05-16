use api_models::utils::Uuid;
use chrono::{DateTime, TimeZone, Utc};
use sqlx::types::ipnetwork::IpNetwork;

use crate::{query, query_as, Database};

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
			token_id UUID CONSTRAINT user_api_token_pk PRIMARY KEY,
			name TEXT NOT NULL,
			user_id UUID NOT NULL,
			token_hash TEXT NOT NULL,
			token_nbf TIMESTAMPTZ, /* The token is not valid before this date */
			token_exp TIMESTAMPTZ, /* The token is not valid after this date */
			allowed_ips INET[],
			created TIMESTAMPTZ NOT NULL,
			revoked TIMESTAMPTZ,
			login_type USER_LOGIN_TYPE GENERATED ALWAYS AS ('api_token') STORED,
			CONSTRAINT user_api_token_token_id_user_id_uk UNIQUE(
				token_id, user_id
			),
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
		CREATE TABLE user_api_token_block_resource_permission(
			token_id UUID NOT NULL
				CONSTRAINT user_api_token_block_resource_permission_fk_token_id
					REFERENCES user_api_token(token_id),
			workspace_id UUID NOT NULL,
			resource_id UUID NOT NULL,
			permission_id UUID NOT NULL
				CONSTRAINT user_api_token_block_resource_permission_fk_permission_id
					REFERENCES permission(id),
			CONSTRAINT user_api_token_block_resource_permission_workspace_id_resource_id
				FOREIGN KEY (workspace_id, resource_id)
					REFERENCES resource(owner_id, id),
			CONSTRAINT user_api_token_block_resource_permission_pk 
				PRIMARY KEY(token_id, permission_id, resource_id, workspace_id)
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

pub async fn create_api_token_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
	name: &str,
	user_id: &Uuid,
	token_hash: &str,
	token_nbf: Option<&DateTime<Utc>>,
	token_exp: Option<&DateTime<Utc>>,
	allowed_ips: Option<&[IpNetwork]>,
	created: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_api_token(
				token_id,
				name,
				user_id,
				token_hash,
				token_nbf,
				token_exp,
				allowed_ips,
				created,
				revoked
			)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, NULL);
		"#,
		token_id as _,
		name,
		user_id as _,
		token_hash,
		token_nbf,
		token_exp,
		allowed_ips,
		created,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn add_super_admin_permission_for_api_token(
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

pub async fn add_resource_type_permission_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
	workspace_id: &Uuid,
	resource_type_id: &Uuid,
	permission_id: &Uuid,
) -> Result<(), sqlx::Error> {
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
	.await
	.map(|_| ())
}

pub async fn add_resource_permission_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
	workspace_id: &Uuid,
	resource_id: &Uuid,
	permission_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
			INSERT INTO
				user_api_token_resource_permission(
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
	.await
	.map(|_| ())
}

pub async fn list_active_api_tokens_for_user(
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
			revoked IS NULL;
		"#,
		user_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_all_super_admin_workspace_ids_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			workspace_id as "workspace_id: Uuid"
		FROM
			user_api_token_workspace_super_admin
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.workspace_id)
	.collect();

	Ok(rows)
}

pub async fn get_all_resource_type_permissions_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
) -> Result<Vec<(Uuid, Uuid, Uuid)>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			workspace_id as "workspace_id: Uuid",
			resource_type_id as "resource_type_id: Uuid",
			permission_id as "permission_id: Uuid"
		FROM
			user_api_token_resource_type_permission
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.resource_type_id, row.permission_id))
	.collect();

	Ok(rows)
}

pub async fn get_all_resource_permissions_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
) -> Result<Vec<(Uuid, Uuid, Uuid)>, sqlx::Error> {
	let rows = query!(
		r#"
		SELECT
			workspace_id as "workspace_id: Uuid",
			resource_id as "resource_id: Uuid",
			permission_id as "permission_id: Uuid"
		FROM
			user_api_token_resource_permission
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.resource_id, row.permission_id))
	.collect();

	Ok(rows)
}

pub async fn get_active_user_api_token_by_id(
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
			revoked IS NULL AND
			token_id = $1;
		"#,
		token_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn revoke_user_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
	revoked_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user_api_token
		SET
			revoked = $2
		WHERE
			token_id = $1 AND
			revoked IS NULL;
		"#,
		token_id as _,
		revoked_time,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_token_hash_for_user_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
	token_hash: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user_api_token
		SET
			token_hash = $1
		WHERE
			token_id = $2 AND
			revoked IS NULL;
		"#,
		token_hash,
		token_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_super_admin_permissions_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			user_api_token_workspace_super_admin
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_resource_type_permissions_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			user_api_token_resource_type_permission
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn remove_all_resource_permissions_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			user_api_token_resource_permission
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_user_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
	name: Option<&str>,
	token_nbf: Option<&DateTime<Utc>>,
	token_exp: Option<&DateTime<Utc>>,
	allowed_ips: Option<&[IpNetwork]>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user_api_token
		SET
			name = COALESCE($1, name),
			token_nbf = (
				CASE
					WHEN $3::TIMESTAMPTZ = $2 THEN
						NULL
					ELSE
						$3
				END
			),
			token_exp = (
				CASE
					WHEN $4::TIMESTAMPTZ = $2 THEN
						NULL
					ELSE
						$4
				END
			),
			allowed_ips = (
				CASE
					WHEN ARRAY_LENGTH($5::INET[], 1) = 0 THEN
						NULL
					ELSE
						$5
				END
			)
		WHERE
			token_id = $6;
		"#,
		name,
		Utc.timestamp_opt(0, 0).unwrap(),
		token_nbf,
		token_exp,
		allowed_ips,
		token_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn revoke_all_expired_user_tokens(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			user_api_token
		SET
			revoked = token_exp
		WHERE
			token_exp > NOW();
		"#
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
