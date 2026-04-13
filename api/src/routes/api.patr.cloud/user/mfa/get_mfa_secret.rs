use axum::http::StatusCode;
use models::api::user::*;
use rustis::commands::{SetExpiration, StringCommands};
use time::Duration;
use totp_rs::{Algorithm as TotpAlgorithm, Builder, Secret};

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
		user_data.id
	)
	.fetch_one(&mut **database)
	.await?;

	if mfa_detail.mfa_secret.is_some() {
		return Err(ErrorType::MfaAlreadyActive);
	}

	let secret = Secret::generate().to_base32();

	let qr = Builder::new()
		.with_algorithm(TotpAlgorithm::SHA1)
		.with_digits(6)
		.with_skew(1)
		.with_step_duration(30)
		.with_secret(Secret::try_from_base32(secret.clone()).inspect_err(|err| {
			error!(
				"Unable to parse MFA secret for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
		})?)
		.with_issuer(Some(constants::TOTP_ISSUER))
		.with_account_name(
			user_data
				.identity
				.email()
				.ok_or(ErrorType::Unauthorized)?
				.to_string(),
		)
		.build()
		.inspect_err(|err| {
			error!(
				"Unable to parse TOTP for userId `{}`: {}",
				user_data.id,
				err.to_string()
			);
		})?
		.to_qr_base64()
		.map_err(ErrorType::server_error)?;

	redis
		.set_with_options(
			redis::user_mfa_secret(&user_data.id),
			secret.clone(),
			None,
			SetExpiration::Ex(Duration::minutes(5).whole_seconds().unsigned_abs()),
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
