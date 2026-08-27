use argon2::{
	Algorithm,
	PasswordHash,
	PasswordHasher,
	PasswordVerifier,
	Version,
	password_hash::generate_salt,
};
use axum::http::StatusCode;
use models::api::auth::*;
use rand::RngExt;

use crate::prelude::*;

pub async fn resend_otp(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: ResendOtpPath,
				query: (),
				headers: ResendOtpRequestHeaders { user_agent: _ },
				body: ResendOtpRequestProcessed { email, password },
			},
		database,
		redis: _,
		client_ip: _,
		state,
	}: AppRequest<'_, ResendOtpRequest>,
) -> Result<AppResponse<ResendOtpRequest>, ErrorType> {
	info!("Resending OTP to email: `{email}`");

	let row = query!(
		r#"
		SELECT
			*
		FROM
			user_to_sign_up
		WHERE
			email = $1::CITEXT;
		"#,
		&email
	)
	.fetch_optional(&mut **database)
	.await?;

	if let Some(user_data) = row {
		let success = argon2::Argon2::new_with_secret(
			state.config.password_pepper.as_ref(),
			Algorithm::Argon2id,
			Version::V0x13,
			constants::HASHING_PARAMS,
		)
		.inspect_err(|err| {
			error!("Error while creating Argon2 instance: {}", err);
		})
		.map_err(ErrorType::server_error)?
		.verify_password(
			password.as_bytes(),
			&PasswordHash::new(&user_data.password).map_err(ErrorType::server_error)?,
		)
		.inspect_err(|err| {
			info!("Error while verifying password: {}", err);
		})
		.is_ok();

		if success {
			let otp = format!("{:06}", rand::rng().random_range(constants::OTP_RANGE));
			let hashed_otp = argon2::Argon2::new_with_secret(
				state.config.password_pepper.as_ref(),
				Algorithm::Argon2id,
				Version::V0x13,
				constants::HASHING_PARAMS,
			)
			.inspect_err(|err| {
				error!("Error while creating Argon2 instance: {}", err);
			})
			.map_err(ErrorType::server_error)?
			.hash_password_with_salt(otp.as_bytes(), &generate_salt())
			.inspect_err(|err| {
				error!("Error hashing OTP: {}", err);
			})
			.map_err(ErrorType::server_error)?
			.to_string();

			query!(
				r#"
				UPDATE
					user_to_sign_up
				SET
					otp_hash = $1
				WHERE
					email = $2::CITEXT;
				"#,
				hashed_otp,
				&email
			)
			.execute(&mut **database)
			.await?;

			// TODO send OTP to user
		}
	}

	AppResponse::builder()
		.body(ResendOtpResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
