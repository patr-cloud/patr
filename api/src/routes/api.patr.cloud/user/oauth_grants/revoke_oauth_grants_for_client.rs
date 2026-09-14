use axum::http::StatusCode;
use models::api::user::*;

use crate::{models::oauth, prelude::*};

pub async fn revoke_oauth_grants_for_client(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: RevokeOAuthGrantsForClientPath { client_id },
				query: (),
				headers:
					RevokeOAuthGrantsForClientRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: RevokeOAuthGrantsForClientRequestProcessed,
			},
		database,
		redis,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, RevokeOAuthGrantsForClientRequest>,
) -> Result<AppResponse<RevokeOAuthGrantsForClientRequest>, ErrorType> {
	trace!(
		"Revoking every `{}` grant for user {}",
		client_id, user_data.id
	);

	let grants = query!(
		r#"
		SELECT
			login_id AS "login_id: Uuid"
		FROM
			oauth_login
		WHERE
			user_id = $1 AND
			client_id = $2 AND
			revoked IS NULL;
		"#,
		user_data.id as _,
		client_id,
	)
	.fetch_all(&mut **database)
	.await?;

	if grants.is_empty() {
		return Err(ErrorType::ResourceDoesNotExist);
	}

	// One at a time rather than a bulk UPDATE, so each grant goes through the
	// single revocation path and gets its Redis state cleared with it.
	for grant in grants {
		oauth::revoke_grant(&mut **database, redis, &grant.login_id).await?;
	}

	AppResponse::builder()
		.status_code(StatusCode::ACCEPTED)
		.headers(())
		.body(RevokeOAuthGrantsForClientResponse)
		.build()
		.into_result()
}
