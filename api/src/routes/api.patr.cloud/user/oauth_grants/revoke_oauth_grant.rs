use axum::http::StatusCode;
use models::api::user::*;

use crate::{models::oauth, prelude::*};

pub async fn revoke_oauth_grant(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: RevokeOAuthGrantPath { grant_id },
				query: (),
				headers:
					RevokeOAuthGrantRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: RevokeOAuthGrantRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, RevokeOAuthGrantRequest>,
) -> Result<AppResponse<RevokeOAuthGrantRequest>, ErrorType> {
	trace!(
		"Revoking OAuth grant `{}` for user {}",
		grant_id, user_data.id
	);

	// Scoped to the caller before anything is revoked. Without the `user_id`
	// check any user could end anyone else's session by guessing a grant id.
	let owned = query!(
		r#"
		SELECT
			login_id
		FROM
			oauth_login
		WHERE
			login_id = $1 AND
			user_id = $2 AND
			revoked IS NULL;
		"#,
		grant_id as _,
		user_data.id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.is_some();

	if !owned {
		return Err(ErrorType::ResourceDoesNotExist);
	}

	oauth::revoke_grant(&mut **database, redis, &grant_id).await?;

	AppResponse::builder()
		.status_code(StatusCode::ACCEPTED)
		.headers(())
		.body(RevokeOAuthGrantResponse)
		.build()
		.into_result()
}
