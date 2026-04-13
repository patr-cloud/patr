use axum::http::StatusCode;
use models::api::user::*;
use rustis::commands::StringCommands;
use time::Duration;
use totp_rs::{Algorithm as TotpAlgorithm, Secret, TOTP};

use crate::{prelude::*, redis::keys as redis};

pub async fn get_mfa_secret(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetMfaSecretPath,
				query: (),
				headers:
					GetMfaSecretRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetMfaSecretRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		state: _,
		user_data,
	}: AuthenticatedAppRequest<'_, GetMfaSecretRequest>,
) -> Result<AppResponse<GetMfaSecretRequest>, ErrorType> {
	info!("Getting MFA secret");

	let mfa_detail = query!(
		r#"
		SELECT
			"user".mfa_secret
		FROM
			"user"
		WHERE
			id = $1;
		"#,
		user_data.id as _
	)
	.fetch_one(&mut **database)
	.await?;

	if mfa_detail.mfa_secret.is_some() {
		return Err(ErrorType::MfaAlreadyActive);
	}

	let secret = Secret::generate_secret().to_encoded().to_string();

	let qr = TOTP::new(
		TotpAlgorithm::SHA1,
		6,
		1,
		30,
		Secret::Encoded(secret.clone())
			.to_bytes()
			.inspect_err(|err| {
				error!(
					"Unable to parse MFA secret for userId `{}`: {}",
					user_data.id,
					err.to_string()
				);
			})?,
		Some(constants::TOTP_ISSUER.to_string()),
		user_data
			.identity
			.email()
			.ok_or(ErrorType::Unauthorized)?
			.to_string(),
	)
	.inspect_err(|err| {
		error!(
			"Unable to parse TOTP for userId `{}`: {}",
			user_data.id,
			err.to_string()
		);
	})?
	.get_qr_base64()
	.map_err(ErrorType::server_error)?;

	redis
		.setex(
			redis::user_mfa_secret(&user_data.id),
			Duration::minutes(5).whole_seconds().unsigned_abs(),
			secret.clone(),
		)
		.await
		.inspect_err(|err| {
			error!(
				"Error setting the MFA secret for user `{}`: `{}`",
				user_data.id, err
			);
		})?;

	AppResponse::builder()
		.body(GetMfaSecretResponse { qr })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
