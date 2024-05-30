use axum::http::StatusCode;
use models::api::user::*;

use crate::prelude::*;

pub async fn update_user_info(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: UpdateUserInfoPath,
				query: (),
				headers:
					UpdateUserInfoRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: UpdateUserInfoRequestProcessed {
					first_name,
					last_name,
				},
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
		user_data,
	}: AuthenticatedAppRequest<'_, UpdateUserInfoRequest>,
) -> Result<AppResponse<UpdateUserInfoRequest>, ErrorType> {
	info!("Updating user info");

	query!(
		r#"
		UPDATE
			"user"
		SET
			first_name = COALESCE($1, first_name),
			last_name = COALESCE($2, last_name)
		WHERE
			id = $3;
		"#,
		first_name,
		last_name,
		user_data.id as _,
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(UpdateUserInfoResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
