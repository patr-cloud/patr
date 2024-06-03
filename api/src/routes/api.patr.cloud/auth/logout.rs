use argon2::{Algorithm, PasswordHash, PasswordVerifier, Version};
use axum::http::StatusCode;
use models::api::auth::*;
use rustis::commands::{GenericCommands, StringCommands};
use time::OffsetDateTime;

use crate::{prelude::*, redis::keys as redis};

pub async fn logout(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: LogoutPath,
				query: (),
				headers: LogoutRequestHeaders {
					refresh_token,
					user_agent: _,
				},
				body: LogoutRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data,
		config,
	}: AuthenticatedAppRequest<'_, LogoutRequest>,
) -> Result<AppResponse<LogoutRequest>, ErrorType> {
	info!("Logging out user: {}", user_data.id);

	// User agent being a browser is expected to be checked in the
	// UserAgentValidationLayer

	let Some((login_id, refresh_token)) = refresh_token.0.token().split_once('.') else {
		return Err(ErrorType::MalformedRefreshToken);
	};

	let login_id = Uuid::parse_str(login_id).map_err(|_| ErrorType::MalformedRefreshToken)?;

	let Some(login) = query!(
		r#"
		SELECT
			web_login.refresh_token,
			web_login.token_expiry
		FROM
			web_login
		WHERE
			login_id = $1;
		"#,
		login_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	else {
		debug!("Could not find a login with the login id: {}", login_id);
		return Err(ErrorType::MalformedRefreshToken);
	};

	let success = argon2::Argon2::new_with_secret(
		config.password_pepper.as_bytes(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.verify_password(
		refresh_token.as_ref(),
		&PasswordHash::new(&login.refresh_token).map_err(ErrorType::server_error)?,
	)
	.inspect_err(|err| {
		info!("Error verifying password: `{}`", err);
	})
	.is_ok();

	if !success {
		return Err(ErrorType::MalformedRefreshToken);
	}

	query!(
		r#"
		DELETE FROM
			web_login
		WHERE
			login_id = $1;
		"#,
		login_id as _,
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted web login");

	query!(
		r#"
		DELETE FROM
			user_login
		WHERE
			login_id = $1;
		"#,
		login_id as _,
	)
	.execute(&mut **database)
	.await?;

	trace!("Deleted user login");

	_ = redis
		.del(redis::permission_for_login_id(&login_id))
		.await
		.inspect_err(|err| {
			error!(
				"Error deleting the cached permission for login `{}`: `{}`",
				login_id, err
			);
		});
	redis
		.setex(
			redis::login_id_revocation_timestamp(&login_id),
			constants::CACHED_PERMISSIONS_VALIDITY.whole_seconds() as u64 + 100,
			OffsetDateTime::now_utc().unix_timestamp(),
		)
		.await
		.inspect_err(|err| {
			error!("Error setting the revocation timestamp: `{}`", err);
		})?;

	AppResponse::builder()
		.body(LogoutResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
