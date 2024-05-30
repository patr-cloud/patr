use axum::http::StatusCode;
use models::api::{user::*, WithId};

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
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, GetUserInfoRequest>,
) -> Result<AppResponse<GetUserInfoRequest>, ErrorType> {
	info!("Getting authenticated user info");

	let row = query!(
		r#"
		SELECT
			"user".username,
			"user".first_name,
			"user".last_name,
			"user".created,
			"user".mfa_secret,
			"user".recovery_phone_country_code,
			"user".recovery_phone_number,
			"user".recovery_email
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
				username: row.username,
				first_name: row.first_name,
				last_name: row.last_name,
			},
		),
		created: row.created,
		is_mfa_enabled: row.mfa_secret.is_some(),
		recovery_email: row.recovery_email,
		recovery_phone_number: row
			.recovery_phone_country_code
			.zip(row.recovery_phone_number)
			.map(|(country_code, phone_number)| UserPhoneNumber {
				country_code,
				phone_number,
			}),
	};

	AppResponse::builder()
		.body(user_info)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
