use axum::http::StatusCode;
use models::api::{WithId, user::*};
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn get_user_info(
	AppRequest {
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
		change_publisher: _,
		config: _,
	}: AppRequest<'_, GetUserInfoRequest>,
) -> Result<AppResponse<GetUserInfoRequest>, ErrorType> {
	info!("Getting authenticated user info");

	let rows = query(
		r#"
		SELECT
			*
		FROM
			meta_data
		WHERE
			id IN (
				$1,
				$2,
				$4
			);
		"#,
	)
	.bind(constants::USER_ID_KEY)
	.bind(constants::FIRST_NAME_KEY)
	.bind(constants::LAST_NAME_KEY)
	.fetch_all(&mut **database)
	.await?;

	let mut db_user_id = None;
	let mut db_first_name = None;
	let mut db_last_name = None;

	for row in rows {
		let id = row.try_get::<String, _>("id")?;
		let value = row.try_get::<String, _>("value")?;

		match id.as_str() {
			constants::USER_ID_KEY => {
				db_user_id = Some(value);
			}
			constants::FIRST_NAME_KEY => {
				db_first_name = Some(value);
			}
			constants::LAST_NAME_KEY => {
				db_last_name = Some(value);
			}
			_ => (),
		}
	}

	let (Some(username), Some(first_name), Some(last_name)) =
		(db_user_id, db_first_name, db_last_name)
	else {
		return Err(ErrorType::UserNotFound);
	};

	let user_info = GetUserInfoResponse {
		basic_user_info: WithId::new(
			Uuid::nil(),
			BasicUserInfo {
				username,
				first_name,
				last_name,
			},
		),
		created: OffsetDateTime::UNIX_EPOCH,
		is_mfa_enabled: false,
		recovery_email: None,
		recovery_phone_number: None,
	};

	AppResponse::builder()
		.body(user_info)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
