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
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, RevokeApiTokenRequest>,
) -> Result<AppResponse<RevokeApiTokenRequest>, ErrorType> {
	trace!("Revoke API token: {}", token_id);

	let rows_affected = query!(
		r#"
		UPDATE
			user_api_token
		SET
			revoked = NOW()
		WHERE
			token_id = $1 AND
			user_id = $2;
		"#,
		token_id as _,
		user_data.id as _,
	)
	.execute(&mut **database)
	.await?
	.rows_affected();

	if rows_affected == 0 {
		return Err(ErrorType::ApiTokenDoesNotExist);
	}

	redis
		.del(redis::keys::permission_for_login_id(&token_id))
		.await?;

	AppResponse::builder()
		.status_code(StatusCode::ACCEPTED)
		.headers(())
		.body(RevokeApiTokenResponse)
		.build()
		.into_result()
}
