use std::ops::Add;

use argon2::{Algorithm, PasswordHasher, Version, password_hash::generate_salt};
use axum::http::StatusCode;
use models::api::auth::*;
use rand::RngExt;
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn resend_otp(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: ResendOtpPath,
				query: (),
				headers: ResendOtpRequestHeaders { user_agent: _ },
				body: ResendOtpRequestProcessed { email },
			},
		database,
		redis: _,
		client_ip: _,
		mut state,
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
		let otp_expiry = OffsetDateTime::now_utc().add(constants::OTP_VALIDITY);

		query!(
			r#"
			UPDATE
				user_to_sign_up
			SET
				otp_hash = $1,
				otp_expiry = $2
			WHERE
				email = $3::CITEXT;
			"#,
			hashed_otp,
			otp_expiry,
			&email
		)
		.execute(&mut **database)
		.await?;

		state
			.worker
			.send_email(
				email.to_string(),
				UserSignUpEmail {
					first_name: user_data.first_name,
					email: email.to_string(),
					otp,
					otp_expiry: constants::OTP_VALIDITY.to_string(),
				},
			)
			.await
			.inspect_err(|err| {
				error!("Error enqueuing sign-up email: `{}`", err);
			})?;
	}

	AppResponse::builder()
		.body(ResendOtpResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
