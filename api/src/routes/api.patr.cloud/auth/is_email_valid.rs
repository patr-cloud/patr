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
		state: _,
	}: AppRequest<'_, IsEmailValidRequest>,
) -> Result<AppResponse<IsEmailValidRequest>, ErrorType> {
	info!("Checking for validity of Email: `{email}`");

	let is_user_exists = query!(
		r#"
		SELECT
			id
		FROM
			"user"
		WHERE
			email = $1;
		"#,
		email,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	let is_user_signing_up = query!(
		r#"
		SELECT
			email
		FROM
			user_to_sign_up
		WHERE
			email = $1 AND
			otp_expiry > NOW();
		"#,
		email,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	AppResponse::builder()
		.body(IsEmailValidResponse {
			available: !is_user_exists && !is_user_signing_up,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
