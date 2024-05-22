use argon2::{Algorithm, PasswordHash, PasswordVerifier, Version};
use axum::http::StatusCode;
use models::api::user::*;
use totp_rs::{Algorithm as TotpAlgorithm, Secret, TOTP};

use crate::prelude::*;

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
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, ChangePasswordRequest>,
) -> Result<AppResponse<ChangePasswordRequest>, ErrorType> {
	info!("Starting: Change password");

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
		config.password_pepper.as_ref(),
		Algorithm::Argon2id,
		Version::V0x13,
		constants::HASHING_PARAMS,
	)
	.map_err(|err| ErrorType::server_error(err.to_string()))?
	.verify_password(
		current_password.as_bytes(),
		&PasswordHash::new(&row.password)
			.map_err(|err| ErrorType::server_error(err.to_string()))?,
	)
	.is_ok();

	if !success {
		return Err(ErrorType::InvalidPassword);
	}

	if let Some(mfa_secret) = row.mfa_secret {
		let Some(mfa_otp) = mfa_otp else {
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
				ErrorType::server_error(err.to_string())
			})?,
		)
		.map_err(|err| {
			error!(
				"Unable to parse TOTP for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
			ErrorType::server_error(err.to_string())
		})?
		.check_current(&mfa_otp)
		.map_err(|err| {
			error!(
				"System time error while checking TOTP for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
			ErrorType::server_error(err.to_string())
		})?;

		if !mfa_valid {
			return Err(ErrorType::MfaOtpInvalid);
		}
	}

	query!(
		r#"
		UPDATE
			"user"
		SET
			password = $1
		WHERE
			id = $2;
		"#,
		&new_password,
		user_data.id as _,
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(ChangePasswordResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
