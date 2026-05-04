use axum::http::StatusCode;
use models::api::user::*;

use crate::prelude::*;

pub async fn disconnect_social_login(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: DisconnectSocialLoginPath { provider },
				query: (),
				headers:
					DisconnectSocialLoginRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DisconnectSocialLoginRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data,
		state: _,
	}: AuthenticatedAppRequest<'_, DisconnectSocialLoginRequest>,
) -> Result<AppResponse<DisconnectSocialLoginRequest>, ErrorType> {
	trace!("Disconnecting {} from user {}", provider, user_data.id);

	// `RETURNING user_id` lets us 404 cleanly when the row didn't exist —
	// otherwise a no-op DELETE looks the same as a successful one.
	query!(
		r#"
		DELETE FROM
			user_social_login
		WHERE
			user_id = $1 AND
			provider = $2
		RETURNING
			user_id;
		"#,
		user_data.id as _,
		provider as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(DisconnectSocialLoginResponse)
		.headers(())
		.status_code(StatusCode::ACCEPTED)
		.build()
		.into_result()
}
