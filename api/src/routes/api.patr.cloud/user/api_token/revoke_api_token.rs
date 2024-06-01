use axum::http::StatusCode;
use models::api::user::*;
use rustis::commands::GenericCommands;

use crate::prelude::*;

pub async fn revoke_api_token(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: RevokeApiTokenPath { token_id },
				query: (),
				headers:
					RevokeApiTokenRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: RevokeApiTokenRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data: _,
		config: _,
	}: AuthenticatedAppRequest<'_, RevokeApiTokenRequest>,
) -> Result<AppResponse<RevokeApiTokenRequest>, ErrorType> {
	trace!("Revoke API token: {}", token_id);

	query!(
		r#"
		UPDATE
			user_api_token
		SET
			revoked = NOW()
		WHERE
			token_id = $1;
		"#,
		token_id as _,
	)
	.execute(&mut **database)
	.await?;

	redis
		.del(redis::keys::permission_for_login_id(&token_id))
		.await?;

	AppResponse::builder()
		.status_code(StatusCode::RESET_CONTENT)
		.headers(())
		.body(RevokeApiTokenResponse)
		.build()
		.into_result()
}
