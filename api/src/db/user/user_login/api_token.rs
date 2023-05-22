use std::collections::{BTreeMap, BTreeSet};

use api_models::{
	models::workspace::{ResourcePermissionType, WorkspacePermission},
	utils::Uuid,
};
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

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "PERMISSION_TYPE", rename_all = "snake_case")]
pub enum PermissionType {
	Include,
	Exclude,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TOKEN_PERMISSION_TYPE", rename_all = "snake_case")]
pub enum TokenPermissionType {
	SuperAdmin,
	Member,
}

pub async fn initialize_api_token_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	// tables are initialized in post due to workspace dependency

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
		CREATE TYPE TOKEN_PERMISSION_TYPE AS ENUM(
			'super_admin',
			'member'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

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
		CREATE TABLE user_api_token_permission_type(
			token_id 				UUID 					NOT NULL,
			workspace_id			UUID 					NOT NULL,
			token_permission_type 	TOKEN_PERMISSION_TYPE 	NOT NULL,

			CONSTRAINT user_api_token_permission_type_pk
				PRIMARY KEY (token_id, workspace_id),

			CONSTRAINT user_api_token_permission_type_uq
				UNIQUE (token_id, workspace_id, token_permission_type),
			
			CONSTRAINT user_api_token_permission_type_fk_token
				FOREIGN KEY(token_id)
					REFERENCES user_api_token(token_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_workspace_super_admin(
			token_id 		UUID NOT NULL,
			user_id 		UUID NOT NULL,
			workspace_id	UUID NOT NULL,

			token_permission_type TOKEN_PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('super_admin') STORED,

			CONSTRAINT user_api_token_workspace_super_admin_pk
				PRIMARY KEY (token_id, user_id, workspace_id),

			CONSTRAINT user_api_token_workspace_super_admin_fk_type
				FOREIGN KEY(token_id, workspace_id, token_permission_type)
					REFERENCES user_api_token_permission_type(token_id, workspace_id, token_permission_type),
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
		CREATE TABLE user_api_token_resource_permissions_type(
			token_id 					UUID 			NOT NULL,
			workspace_id 				UUID 			NOT NULL,
			permission_id 				UUID 			NOT NULL,
			resource_permission_type	PERMISSION_TYPE NOT NULL,

			token_permission_type TOKEN_PERMISSION_TYPE NOT NULL
				GENERATED ALWAYS AS ('member') STORED,

			CONSTRAINT user_api_token_resource_permissions_type_pk 
				PRIMARY KEY(token_id, workspace_id, permission_id),

			CONSTRAINT user_api_token_resource_permissions_type_uq
				UNIQUE (token_id, workspace_id, permission_id, resource_permission_type),

			CONSTRAINT user_api_token_resource_permisssions_type_fk_type
				FOREIGN KEY (token_id, workspace_id, token_permission_type)
					REFERENCES user_api_token_permission_type(token_id, workspace_id, token_permission_type),
			CONSTRAINT user_api_token_resource_permisssions_type_fk_permission_id
				FOREIGN KEY (permission_id)
					REFERENCES permission(id)
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
				FOREIGN KEY (token_id, workspace_id, permission_id, permission_type)
					REFERENCES user_api_token_resource_permissions_type(token_id, workspace_id, permission_id, resource_permission_type),
			CONSTRAINT user_api_token_resource_permissions_include_fk_resource
				FOREIGN KEY (resource_id, workspace_id)
					REFERENCES resource(id, owner_id)			
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
				FOREIGN KEY (token_id, workspace_id, permission_id, permission_type)
					REFERENCES user_api_token_resource_permissions_type(token_id, workspace_id, permission_id, resource_permission_type),
			CONSTRAINT user_api_token_resource_permissions_exclude_fk_resource
				FOREIGN KEY (resource_id, workspace_id)
					REFERENCES resource(id, owner_id)			
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

pub async fn insert_raw_permissions_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
	user_id: &Uuid,
	workspace_permissions: &BTreeMap<Uuid, WorkspacePermission>,
) -> Result<(), sqlx::Error> {
	for (workspace_id, workspace_permission) in workspace_permissions {
		match workspace_permission {
			WorkspacePermission::SuperAdmin => {
				query!(
					r#"
					INSERT INTO
						user_api_token_permission_type(
							token_id,
							workspace_id,
							token_permission_type
						)
					VALUES
						($1, $2, $3);
					"#,
					token_id as _,
					workspace_id as _,
					TokenPermissionType::SuperAdmin as _,
				)
				.execute(&mut *connection)
				.await?;

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
				.await?;
			}
			WorkspacePermission::Member {
				permissions: member_permissions,
			} => {
				query!(
					r#"
					INSERT INTO
						user_api_token_permission_type(
							token_id,
							workspace_id,
							token_permission_type
						)
					VALUES
						($1, $2, $3);
					"#,
					token_id as _,
					workspace_id as _,
					TokenPermissionType::Member as _,
				)
				.execute(&mut *connection)
				.await?;

				for (permission_id, resource_permission_type) in
					member_permissions
				{
					match resource_permission_type {
						ResourcePermissionType::Include(resource_ids) => {
							query!(
								r#"
								INSERT INTO
									user_api_token_resource_permissions_type(
										token_id,
										workspace_id,
										permission_id,
										resource_permission_type
									)
								VALUES
									($1, $2, $3, $4);
								"#,
								token_id as _,
								workspace_id as _,
								permission_id as _,
								PermissionType::Include as _,
							)
							.execute(&mut *connection)
							.await?;

							for resource_id in resource_ids {
								query!(
									r#"
									INSERT INTO
										user_api_token_resource_permissions_include(
											token_id,
											workspace_id,
											permission_id,
											resource_id
										)
									VALUES
										($1, $2, $3, $4);
									"#,
									token_id as _,
									workspace_id as _,
									permission_id as _,
									resource_id as _,
								)
								.execute(&mut *connection)
								.await?;
							}
						}
						ResourcePermissionType::Exclude(resource_ids) => {
							query!(
								r#"
								INSERT INTO
									user_api_token_resource_permissions_type(
										token_id,
										workspace_id,
										permission_id,
										resource_permission_type
									)
								VALUES
									($1, $2, $3, $4);
								"#,
								token_id as _,
								workspace_id as _,
								permission_id as _,
								PermissionType::Exclude as _,
							)
							.execute(&mut *connection)
							.await?;

							for resource_id in resource_ids {
								query!(
									r#"
									INSERT INTO
										user_api_token_resource_permissions_exclude(
											token_id,
											workspace_id,
											permission_id,
											resource_id
										)
									VALUES
										($1, $2, $3, $4);
									"#,
									token_id as _,
									workspace_id as _,
									permission_id as _,
									resource_id as _,
								)
								.execute(&mut *connection)
								.await?;
							}
						}
					}
				}
			}
		}
	}

	Ok(())
}

pub async fn get_raw_permissions_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
) -> Result<BTreeMap<Uuid, WorkspacePermission>, sqlx::Error> {
	let mut workspace_member_permissions =
		BTreeMap::<Uuid, BTreeMap<Uuid, ResourcePermissionType>>::new();

	query!(
		r#"
		SELECT
			workspace_id as "workspace_id: Uuid",
			permission_id as "permission_id: Uuid",
			resource_id as "resource_id: Uuid"
		FROM
			user_api_token_resource_permissions_include
		WHERE
			token_id = $1;
		"#,
		token_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.resource_id))
	.fold(
		BTreeMap::<(Uuid, Uuid), BTreeSet<Uuid>>::new(),
		|mut accu, (workspace_id, permission_id, resource_id)| {
			accu.entry((workspace_id, permission_id))
				.or_default()
				.insert(resource_id);
			accu
		},
	)
	.into_iter()
	.for_each(|((workspace_id, permission_id), resource_ids)| {
		workspace_member_permissions
			.entry(workspace_id)
			.or_default()
			.insert(
				permission_id,
				ResourcePermissionType::Include(resource_ids),
			);
	});

	query!(
		r#"
		SELECT
			workspace_id as "workspace_id: Uuid",
			permission_id as "permission_id: Uuid",
			resource_id as "resource_id: Uuid"
		FROM
			user_api_token_resource_permissions_exclude
		WHERE
			token_id = $1;
		"#,
		token_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.resource_id))
	.fold(
		BTreeMap::<(Uuid, Uuid), BTreeSet<Uuid>>::new(),
		|mut accu, (workspace_id, permission_id, resource_id)| {
			accu.entry((workspace_id, permission_id))
				.or_default()
				.insert(resource_id);
			accu
		},
	)
	.into_iter()
	.for_each(|((workspace_id, permission_id), resource_ids)| {
		workspace_member_permissions
			.entry(workspace_id)
			.or_default()
			.insert(
				permission_id,
				ResourcePermissionType::Exclude(resource_ids),
			);
	});

	query!(
		r#"
		SELECT
			workspace_id as "workspace_id: Uuid",
			permission_id as "permission_id: Uuid",
			resource_permission_type as "permission_type: PermissionType"
		FROM
			user_api_token_resource_permissions_type
		WHERE
			token_id = $1;
		"#,
		token_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.workspace_id, row.permission_id, row.permission_type))
	.for_each(|(workspace_id, permission_id, permission_type)| {
		workspace_member_permissions
			.entry(workspace_id)
			.or_default()
			.entry(permission_id)
			.or_insert_with(|| match permission_type {
				PermissionType::Include => {
					ResourcePermissionType::Include(BTreeSet::new())
				}
				PermissionType::Exclude => {
					ResourcePermissionType::Exclude(BTreeSet::new())
				}
			});
	});

	let mut workspace_permissions = workspace_member_permissions
		.into_iter()
		.map(|(workspace_id, member_permissions)| {
			(
				workspace_id,
				WorkspacePermission::Member {
					permissions: member_permissions,
				},
			)
		})
		.collect::<BTreeMap<_, _>>();

	query!(
		r#"
		SELECT
			workspace_id as "workspace_id: Uuid"
		FROM
			user_api_token_permission_type
		WHERE
			token_permission_type = 'member' AND
			token_id = $1;
		"#,
		token_id as _
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.workspace_id)
	.for_each(|workspace_id| {
		// add workspace member permissions which may be empty
		workspace_permissions
			.entry(workspace_id)
			.or_insert_with(|| WorkspacePermission::Member {
				permissions: BTreeMap::new(),
			});
	});

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
	.map(|row| row.workspace_id)
	.for_each(|workspace_id| {
		// override the member roles for workspace with super admin
		// as superadmin has high privilege than member permissions
		workspace_permissions
			.insert(workspace_id, WorkspacePermission::SuperAdmin);
	});

	Ok(workspace_permissions)
}

pub async fn remove_all_permissions_for_api_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	token_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		DELETE FROM
			user_api_token_resource_permissions_exclude
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DELETE FROM
			user_api_token_resource_permissions_include
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DELETE FROM
			user_api_token_resource_permissions_type
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut *connection)
	.await?;

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
	.await?;

	query!(
		r#"
		DELETE FROM
			user_api_token_permission_type
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
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
	remove_all_permissions_for_api_token(&mut *connection, token_id).await?;

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
	let revoked_tokens = query!(
		r#"
		UPDATE
			user_api_token
		SET
			revoked = token_exp
		WHERE
			token_exp > NOW()
		RETURNING
			token_id as "token_id: Uuid";
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.token_id);

	for token_id in revoked_tokens {
		remove_all_permissions_for_api_token(&mut *connection, &token_id)
			.await?;
	}

	Ok(())
}
