use axum::http::StatusCode;
use models::api::auth::*;

use crate::prelude::*;

pub async fn is_username_valid(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: IsUsernameValidPath,
				query: IsUsernameValidQuery { username },
				headers: IsUsernameValidRequestHeaders { user_agent: _ },
				body: IsUsernameValidRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
	}: AppRequest<'_, IsUsernameValidRequest>,
) -> Result<AppResponse<IsUsernameValidRequest>, ErrorType> {
	info!("Checking for validity of username: `{username}`");

	// User agent being a browser is expected to be checked in the
	// UserAgentValidationLayer

	let is_user_exists = query!(
		r#"
		SELECT
			*
		FROM
			"user"
		WHERE
			username = $1;
		"#,
		username,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	trace!("Does the user exist: {is_user_exists}");

	let is_user_signing_up = query!(
		r#"
		SELECT
			*
		FROM
			user_to_sign_up
		WHERE
			username = $1 AND
			otp_expiry > NOW();
		"#,
		username,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	trace!("Is the user going to sign up: {is_user_signing_up}");

	AppResponse::builder()
		.body(IsUsernameValidResponse {
			available: !is_user_exists && !is_user_signing_up,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
