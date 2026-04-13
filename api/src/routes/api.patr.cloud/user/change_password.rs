use argon2::{
	Algorithm,
	PasswordHash,
	PasswordHasher,
	PasswordVerifier,
	Version,
	password_hash::generate_salt,
};
use axum::http::StatusCode;
use models::api::user::*;
use rustis::commands::StringCommands as _;
use time::OffsetDateTime;
use totp_rs::{Algorithm as TotpAlgorithm, Secret, TOTP};

use crate::{prelude::*, redis::keys as redis};

pub async fn change_password(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ChangePasswordPath,
				query: (),
				headers:
					ChangePasswordRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					ChangePasswordRequestProcessed {
						current_password,
						new_password,
						mfa_otp,
					},
			},
		database,
		redis,
		client_ip: _,
		user_data,
		state,
	}: AuthenticatedAppRequest<'_, ChangePasswordRequest>,
) -> Result<AppResponse<ChangePasswordRequest>, ErrorType> {
	info!("Changing user password");

	let row = query!(
		r#"
		SELECT
			password,
			mfa_secret
		FROM
			"user"
		WHERE
			id = $1;
		"#,
		user_data.id as _
	)
	.fetch_one(&mut **database)
	.await?;

	let success = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2 instance: {err}");
	})
	.map_err(ErrorType::server_error)?
	.verify_password(
		current_password.as_bytes(),
		&PasswordHash::new(&row.password).map_err(ErrorType::server_error)?,
	)
	.inspect_err(|err| {
		error!("Error verifying password: {err}");
	})
	.is_ok();

	if !success {
		return Err(ErrorType::InvalidPassword);
	}

	if current_password == new_password {
		return Err(ErrorType::InvalidPassword);
	}

	if let Some(mfa_secret) = row.mfa_secret {
		let Some(mfa_otp) = mfa_otp else {
			debug!("MFA required for userId `{}`", user_data.id);
			return Err(ErrorType::MfaRequired);
		};

		let mfa_valid = TOTP::new(
			TotpAlgorithm::SHA1,
			6,
			1,
			30,
			Secret::Encoded(mfa_secret).to_bytes().map_err(|err| {
				error!(
					"Unable to parse MFA secret for userId `{}`: {}",
					user_data.id,
					err.to_string()
				);
				ErrorType::server_error(err)
			})?,
			Some(constants::TOTP_ISSUER.to_string()),
			user_data
				.identity
				.username()
				.ok_or(ErrorType::Unauthorized)?
				.to_string(),
		)
		.inspect_err(|err| {
			error!(
				"Unable to parse TOTP for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
		})
		.map_err(ErrorType::server_error)?
		.check_current(&mfa_otp)
		.inspect_err(|err| {
			error!(
				"System time error while checking TOTP for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
		})
		.map_err(ErrorType::server_error)?;

		if !mfa_valid {
			info!("MFA OTP invalid for userId `{}`", user_data.id);
			return Err(ErrorType::MfaOtpInvalid);
		}
	}

	let hashed_password = argon2::Argon2::new_with_secret(
		state.config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.inspect_err(|err| {
		error!("Error creating Argon2: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.hash_password_with_salt(new_password.as_bytes(), &generate_salt())
	.inspect_err(|err| {
		error!("Error hashing password: `{}`", err);
	})
	.map_err(ErrorType::server_error)?
	.to_string();

	query!(
		r#"
		UPDATE
			"user"
		SET
			password = $1
		WHERE
			id = $2;
		"#,
		&hashed_password,
		user_data.id as _,
	)
	.execute(&mut **database)
	.await?;

	trace!("Password updated for userId `{}`", user_data.id);

	// Drop every other web login the user has — the password the attacker
	// used to mint them is now invalid for the refresh path. Keep the
	// caller's session so the success UX can land.
	query!(
		r#"
		DELETE FROM
			web_login
		WHERE
			user_id = $1 AND
			login_id != $2;
		"#,
		user_data.id as _,
		user_data.login_id as _,
	)
	.execute(&mut **database)
	.await?;

	redis
		.setex(
			redis::user_id_revocation_timestamp(&user_data.id.into()),
			constants::CACHED_PERMISSIONS_VALIDITY
				.whole_seconds()
				.unsigned_abs(),
			OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
		)
		.await
		.inspect_err(|err| {
			error!("Error setting user_id_revocation_timestamp: `{}`", err);
		})?;

	AppResponse::builder()
		.body(ChangePasswordResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
