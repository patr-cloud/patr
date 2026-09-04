use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use models::{
	api::user::*,
	rbac::{ResourcePermissionTypeDiscriminant, WorkspacePermission},
};
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn create_api_token(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateApiTokenPath,
				query: (),
				headers:
					CreateApiTokenRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					CreateApiTokenRequestProcessed {
						token:
							UserApiTokenProcessed {
								name,
								permissions,
								token_nbf,
								token_exp,
								allowed_ips,
								created: _,
							},
					},
			},
		database,
		redis: _,
		client_ip: _,
		user_data,
		state,
	}: AuthenticatedAppRequest<'_, CreateApiTokenRequest>,
) -> Result<AppResponse<CreateApiTokenRequest>, ErrorType> {
	info!("Creating API token");

	if permissions.is_empty() {
		return Err(ErrorType::WrongParameters);
	}

	if let (Some(nbf), Some(exp)) = (token_nbf, token_exp) {
		if nbf > exp {
			return Err(ErrorType::WrongParameters);
		}
	}

	// An empty whitelist would otherwise permanently lock the token (the
	// authenticator's `.any(...)` over an empty list always returns false).
	// Treat `[]` as "no whitelist" — semantically the user wanted no IP
	// restriction anyway, and we don't get a footgun in the DB.
	let allowed_ips = allowed_ips.filter(|ips| !ips.is_empty());

	let now = OffsetDateTime::now_utc();

	let refresh_token = Uuid::new_v4();
	let hashed_refresh_token = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(refresh_token.as_bytes(), &generate_salt())
	.inspect_err(|err| {
		error!("Error hashing refresh token: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.to_string();

	let token_id = query!(
		r#"
		WITH client AS (
			INSERT INTO
				actor_client(id, actor_client_type)
			VALUES
				(GENERATE_LOGIN_ID(), 'user_login')
			RETURNING id
		)
		INSERT INTO
			user_login(
				login_id,
				user_id,
				login_type,
				created
			)
		SELECT
			client.id,
			$1,
			'api_token',
			$2
		FROM
			client
		RETURNING user_login.login_id;
		"#,
		user_data.id as _,
		now,
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
			ErrorType::ApiTokenAlreadyExists
		}
		_ => ErrorType::InternalServerError,
	})?
	.login_id
	.into();

	trace!("User login inserted");

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
				revoked,
				login_type
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				$6,
				$7,
				$8,
				NULL,
				DEFAULT
			);
		"#,
		token_id as _,
		&name,
		user_data.id as _,
		&hashed_refresh_token,
		token_nbf,
		token_exp,
		allowed_ips.as_deref(),
		now,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
			ErrorType::ApiTokenAlreadyExists
		}
		other => ErrorType::server_error(other),
	})?;

	trace!("API token inserted");

	for (workspace_id, permission) in permissions {
		trace!("Inserting permission for workspace ID: `{workspace_id}`");

		let Some(user_permission) = user_data.permissions.get(&workspace_id) else {
			debug!("The user does not have any permissions on workspace ID: `{workspace_id}`");
			return Err(ErrorType::Unauthorized);
		};

		if !user_permission.is_superset_of(&permission, workspace_id) {
			debug!("The user does not have adequate permissions on workspace ID: `{workspace_id}`");
			return Err(ErrorType::Unauthorized);
		}

		match permission {
			WorkspacePermission::SuperAdmin => {
				trace!("Inserting permission as super admin");
				query!(
					r#"
					INSERT INTO
						user_api_token_workspace_permission_type(
							token_id,
							workspace_id,
							token_permission_type
						)
					VALUES
						(
							$1,
							$2,
							'super_admin'
						);
					"#,
					token_id as _,
					workspace_id as _,
				)
				.execute(&mut **database)
				.await?;

				query!(
					r#"
					INSERT INTO
						user_api_token_workspace_super_admin(
							token_id,
							user_id,
							workspace_id,
							token_permission_type
						)
					VALUES
						(
							$1,
							$2,
							$3,
							DEFAULT
						);
					"#,
					token_id as _,
					user_data.id as _,
					workspace_id as _,
				)
				.execute(&mut **database)
				.await?;
			}
			WorkspacePermission::Member { permissions } => {
				trace!("Inserting permission as member");
				query!(
					r#"
					INSERT INTO
						user_api_token_workspace_permission_type(
							token_id,
							workspace_id,
							token_permission_type
						)
					VALUES
						(
							$1,
							$2,
							'member'
						);
					"#,
					token_id as _,
					workspace_id as _,
				)
				.execute(&mut **database)
				.await?;

				for (permission_id, scopes) in permissions {
					// A grant at the workspace root is the legacy exclude-type
					// row with no exclusions; anything else is an include list.
					let scoped_to_root = scopes.contains(&workspace_id);
					let legacy_type = if scoped_to_root {
						ResourcePermissionTypeDiscriminant::Exclude
					} else {
						ResourcePermissionTypeDiscriminant::Include
					};
					query!(
						r#"
						INSERT INTO
							user_api_token_resource_permissions_type(
								token_id,
								workspace_id,
								permission_id,
								resource_permission_type,
								token_permission_type
							)
						VALUES
							(
								$1,
								$2,
								$3,
								$4,
								DEFAULT
							);
						"#,
						token_id as _,
						workspace_id as _,
						permission_id as _,
						legacy_type as _,
					)
					.execute(&mut **database)
					.await?;

					if !scoped_to_root {
						query!(
							r#"
							INSERT INTO
								user_api_token_resource_permissions_include(
									token_id,
									workspace_id,
									permission_id,
									resource_id,
									resource_deleted,
									permission_type
								)
							VALUES
								(
									$1,
									$2,
									$3,
									UNNEST($4::UUID[]),
									DEFAULT,
									DEFAULT
								);
							"#,
							token_id as _,
							workspace_id as _,
							permission_id as _,
							&scopes.into_iter().map(Into::into).collect::<Vec<_>>(),
						)
						.execute(&mut **database)
						.await?;
					}
				}
			}
		}
	}

	AppResponse::builder()
		.body(CreateApiTokenResponse {
			id: token_id,
			token: format!("patrv1.{}.{}", refresh_token, token_id),
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
