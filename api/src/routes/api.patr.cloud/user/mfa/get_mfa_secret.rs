use axum::http::StatusCode;
use models::api::user::*;
use rustis::commands::StringCommands;
use time::Duration;
use totp_rs::Secret;

use crate::{prelude::*, redis::keys as redis};

pub async fn get_mfa_secret(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetMfaSecretPath,
				query: (),
				headers: GetMfaSecretRequestHeaders { authorization: _ },
				body: GetMfaSecretRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		config: _,
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

	redis
		.setex(
			redis::user_mfa_secret(&user_data.id),
			Duration::minutes(5).whole_seconds() as u64,
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
		.body(GetMfaSecretResponse { secret })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
