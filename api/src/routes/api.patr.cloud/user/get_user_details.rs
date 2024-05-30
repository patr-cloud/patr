use axum::http::StatusCode;
use models::api::{user::*, WithId};

use crate::prelude::*;

pub async fn get_user_details(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetUserDetailsPath { user_id },
				query: (),
				headers:
					GetUserDetailsRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetUserDetailsRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, GetUserDetailsRequest>,
) -> Result<AppResponse<GetUserDetailsRequest>, ErrorType> {
	info!("Getting user details by UserID");

	let basic_user_info = query!(
		r#"
		SELECT
			"user".username,
			"user".first_name,
			"user".last_name
		FROM
			"user"
		WHERE
			id = $1;
		"#,
		user_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.map(|row| GetUserDetailsResponse {
		basic_user_info: WithId::new(
			user_data.id,
			BasicUserInfo {
				username: row.username,
				first_name: row.first_name,
				last_name: row.last_name,
			},
		),
	})
	.ok_or(ErrorType::UserNotFound)?;

	AppResponse::builder()
		.body(basic_user_info)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
