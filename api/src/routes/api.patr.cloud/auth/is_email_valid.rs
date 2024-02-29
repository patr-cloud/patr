use axum::http::StatusCode;
use models::api::auth::*;

use crate::prelude::*;

pub async fn is_email_valid(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: IsEmailValidPath,
				query: IsEmailValidQuery { email },
				headers: IsEmailValidRequestHeaders { user_agent: _ },
				body: IsEmailValidRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
	}: AppRequest<'_, IsEmailValidRequest>,
) -> Result<AppResponse<IsEmailValidRequest>, ErrorType> {
	info!("Checking for validity of Email: `{email}`");

	// User agent being a browser is expected to be checked in the
	// UserAgentValidationLayer

	let is_user_exists = query!(
		r#"
		SELECT
			*
		FROM
			user_email
		WHERE
			email = $1;
		"#,
		email,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	trace!("Does the user exist: {is_user_exists}");

	let is_user_unverified_exists = query!(
		r#"
		SELECT
			*
		FROM
			user_unverified_email
		WHERE
			email = $1 AND
			verification_token_expiry > NOW();
		"#,
		email,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	trace!("Does the user exist unverified: {is_user_unverified_exists}");

	let is_user_signing_up = query!(
		r#"
		SELECT
			*
		FROM
			user_to_sign_up
		WHERE
			recovery_email = $1 AND
			otp_expiry > NOW();
		"#,
		email,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	trace!("Is the user going to sign up: {is_user_signing_up}");

	AppResponse::builder()
		.body(IsEmailValidResponse {
			available: !is_user_exists && !is_user_unverified_exists && !is_user_signing_up,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
