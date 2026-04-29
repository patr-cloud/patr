use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use models::api::auth::*;
use rustis::commands::StringCommands;
use time::OffsetDateTime;

use super::{GithubSetupPayload, create_session};
use crate::{prelude::*, redis::keys as redis_keys};

/// `POST /auth/social-login/github/setup`
///
/// Creates a new Patr account from a confirmed GitHub identity after the user
/// has reviewed/edited the pre-filled profile details.
pub async fn github_oauth_setup(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: GithubOAuthSetupPath,
				query: (),
				headers: GithubOAuthSetupRequestHeaders { user_agent },
				body:
					GithubOAuthSetupRequestProcessed {
						setup_token,
						username,
						first_name,
						last_name,
					},
			},
		database,
		redis,
		client_ip,
		mut state,
	}: AppRequest<'_, GithubOAuthSetupRequest>,
) -> Result<AppResponse<GithubOAuthSetupRequest>, ErrorType> {
	trace!("Processing GitHub OAuth account setup");

	// Atomically fetch-and-consume the setup token. `GETDEL` ensures two
	// concurrent requests with the same token cannot both observe it as valid.
	let setup_key = redis_keys::social_login_setup(&OAuthProvider::Github, &setup_token);
	let raw = redis
		.getdel::<Option<String>>(&setup_key)
		.await
		.inspect_err(|err| error!("Redis error consuming setup token: {err}"))?
		.ok_or(ErrorType::GithubOAuthFailed)?;

	let payload: GithubSetupPayload =
		serde_json::from_str(&raw).map_err(ErrorType::server_error)?;

	// Require a verified email from GitHub — accounts with no verified email cannot
	// complete setup
	let recovery_email = payload.email.ok_or(ErrorType::GithubOAuthFailed)?;

	// Check username availability
	let username_taken = query!(r#"SELECT id FROM "user" WHERE username = $1;"#, &username,)
		.fetch_optional(&mut **database)
		.await?
		.is_some();

	if username_taken {
		return Err(ErrorType::UsernameUnavailable);
	}

	// Check email availability
	let email_taken = query!(
		r#"
		SELECT id AS "id!" FROM "user" WHERE recovery_email = $1
		UNION
		SELECT user_id AS "id!" FROM user_email WHERE email = $1
		LIMIT 1;
		"#,
		&recovery_email,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if email_taken {
		return Err(ErrorType::EmailUnavailable);
	}

	let now = OffsetDateTime::now_utc();
	let user_id = Uuid::new_v4();

	// Hash a random password (user authenticates via GitHub; can reset via email)
	let random_password = Uuid::new_v4().to_string();
	let hashed_password = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| error!("Error creating Argon2: {err}"))
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(random_password.as_bytes(), &generate_salt())
	.inspect_err(|err| error!("Error hashing password: {err}"))
	.map_err(ErrorType::server_error)?
	.to_string();

	// Defer constraints to allow the circular insert order
	query!(r#"SET CONSTRAINTS ALL DEFERRED;"#)
		.execute(&mut **database)
		.await?;

	query!(
		r#"
		INSERT INTO "user"(
			id, username, password,
			first_name, last_name, created,
			recovery_email, recovery_phone_country_code, recovery_phone_number,
			workspace_limit,
			password_reset_token, password_reset_token_expiry, password_reset_attempts,
			mfa_secret
		) VALUES (
			$1, $2, $3,
			$4, $5, $6,
			$7, NULL, NULL,
			$8,
			NULL, NULL, NULL,
			NULL
		);
		"#,
		user_id as _,
		&username,
		hashed_password,
		&first_name,
		&last_name,
		now,
		&recovery_email,
		constants::DEFAULT_WORKSPACE_LIMIT,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"INSERT INTO user_email(user_id, email) VALUES ($1, $2);"#,
		user_id as _,
		&recovery_email,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO user_social_login(user_id, provider, external_id, linked_at)
		VALUES ($1, 'github', $2, $3);
		"#,
		user_id as _,
		payload.external_id,
		now,
	)
	.execute(&mut **database)
	.await?;

	query!(r#"SET CONSTRAINTS ALL IMMEDIATE;"#)
		.execute(&mut **database)
		.await?;

	let (access_token, refresh_token) = create_session(
		database,
		&mut state,
		redis,
		client_ip,
		user_agent.to_string(),
		user_id,
	)
	.await?;

	AppResponse::builder()
		.body(GithubOAuthSetupResponse {
			access_token,
			refresh_token,
		})
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
