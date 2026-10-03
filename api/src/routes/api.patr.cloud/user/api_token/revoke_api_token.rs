use axum::http::StatusCode;
use models::api::user::*;

use crate::{models::permissions, prelude::*};

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
		actor_data,
		state: _,
	}: AuthenticatedAppRequest<'_, RevokeApiTokenRequest>,
) -> Result<AppResponse<RevokeApiTokenRequest>, ErrorType> {
	trace!("Revoke API token: {}", token_id);

	let Some(token) = query!(
		r#"
		UPDATE
			user_api_token
		SET
			revoked = NOW()
		WHERE
			token_id = $1 AND
			user_id = $2 AND
			revoked IS NULL
		RETURNING
			token_hash;
		"#,
		token_id as _,
		actor_data.id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	else {
		return Err(ErrorType::ApiTokenDoesNotExist);
	};

	permissions::mark_token_stale(redis, &token_id, &token.token_hash).await?;

	AppResponse::builder()
		.status_code(StatusCode::ACCEPTED)
		.headers(())
		.body(RevokeApiTokenResponse)
		.build()
		.into_result()
}
