use axum::http::StatusCode;
use models::api::{auth::SocialLoginProvider, user::*};

use crate::prelude::*;

pub async fn list_social_logins(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: ListSocialLoginsPath,
				query: (),
				headers:
					ListSocialLoginsRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: ListSocialLoginsRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, ListSocialLoginsRequest>,
) -> Result<AppResponse<ListSocialLoginsRequest>, ErrorType> {
	trace!("Listing social logins for user: {}", user_data.id);

	let logins = query!(
		r#"
		SELECT
			provider AS "provider: SocialLoginProvider",
			linked_at
		FROM
			user_social_login
		WHERE
			user_id = $1
		ORDER BY
			linked_at ASC;
		"#,
		user_data.id as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|row| LinkedSocialLogin {
		provider: row.provider,
		linked_at: row.linked_at,
	})
	.collect::<Vec<_>>();

	AppResponse::builder()
		.body(ListSocialLoginsResponse { logins })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
