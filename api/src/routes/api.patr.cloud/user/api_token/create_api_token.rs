use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use models::api::user::*;
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
								super_admin_of,
								grants,
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

	if super_admin_of.is_empty() && grants.is_empty() {
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

	// Super-admin entries: the DB itself enforces that only the workspace's
	// owner can mint these, via the FK to workspace(id, super_admin_id).
	for workspace_id in super_admin_of {
		trace!("Inserting super-admin entry for workspace ID: `{workspace_id}`");

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
		.await
		.map_err(|err| match err {
			sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
				ErrorType::Unauthorized
			}
			other => ErrorType::server_error(other),
		})?;
	}

	// The ceiling rows. Validation is structural only — the composite FKs
	// pin the role and every scope to the named workspace. A ceiling above
	// the owner's current permissions is allowed: the intersection at auth
	// time clamps it, and a later promotion needs no re-mint.
	for (workspace_id, workspace_grants) in grants {
		trace!("Inserting ceiling rows for workspace ID: `{workspace_id}`");

		for grant in workspace_grants {
			query!(
				r#"
				INSERT INTO
					user_api_token_permission_binding(
						token_id, workspace_id, permission_id, scope_id
					)
				VALUES
					($1, $2, $3, $4)
				ON CONFLICT
					(token_id, permission_id, scope_id)
				DO NOTHING;
				"#,
				token_id as _,
				workspace_id as _,
				grant.permission_id as _,
				grant.resource_id as _,
			)
			.execute(&mut **database)
			.await
			.map_err(|err| match err {
				sqlx::Error::Database(db_err) if db_err.is_foreign_key_violation() => {
					match db_err.constraint() {
						Some("user_api_token_permission_binding_fk_permission_id") => {
							ErrorType::WrongParameters
						}
						_ => ErrorType::ResourceDoesNotExist,
					}
				}
				other => ErrorType::server_error(other),
			})?;
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
