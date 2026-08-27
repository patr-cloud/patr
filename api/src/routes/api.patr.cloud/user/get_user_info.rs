use axum::http::StatusCode;
use models::api::{WithId, user::*};

use crate::prelude::*;

pub async fn get_user_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetUserInfoPath,
				query: (),
				headers:
					GetUserInfoRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetUserInfoRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		state: _,
		user_data,
	}: AuthenticatedAppRequest<'_, GetUserInfoRequest>,
) -> Result<AppResponse<GetUserInfoRequest>, ErrorType> {
	info!("Getting authenticated user info");

	let row = query!(
		r#"
		SELECT
			"user".email,
			"user".first_name,
			"user".last_name,
			"user".created,
			"user".mfa_secret
		FROM
			"user"
		WHERE
			"user".id = $1;
		"#,
		user_data.id as _
	)
	.fetch_one(&mut **database)
	.await?;

	let user_info = GetUserInfoResponse {
		basic_user_info: WithId::new(
			user_data.id,
			BasicUserInfo {
				first_name: row.first_name,
				last_name: row.last_name,
			},
		),
		created: row.created,
		is_mfa_enabled: row.mfa_secret.is_some(),
		email: row.email,
	};

	AppResponse::builder()
		.body(user_info)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
