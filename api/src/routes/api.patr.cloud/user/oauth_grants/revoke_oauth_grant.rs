use axum::http::StatusCode;
use models::api::user::*;

use crate::{models::oauth, prelude::*};

/// Revoke one of the user's grants, cutting that app off.
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
		actor_data,
		state: _,
	}: AuthenticatedAppRequest<'_, RevokeOAuthGrantRequest>,
) -> Result<AppResponse<RevokeOAuthGrantRequest>, ErrorType> {
	trace!("Revoking OAuth grant: {}", grant_id);

	query!(
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
		grant_id,
		actor_data.id,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	oauth::revoke_grant(database, redis, &grant_id).await?;

	AppResponse::builder()
		.status_code(StatusCode::ACCEPTED)
		.headers(())
		.body(RevokeOAuthGrantResponse)
		.build()
		.into_result()
}
