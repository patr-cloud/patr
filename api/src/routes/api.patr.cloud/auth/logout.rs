use axum::http::StatusCode;
use models::api::auth::{LogoutPath, LogoutRequest, LogoutResponse};

use crate::prelude::*;

pub async fn logout(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path: LogoutPath,
			query: (),
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
        user_data,
		config,
	}: AuthenticatedAppRequest<'_, LogoutRequest>,
) -> Result<AppResponse<LogoutRequest>, ErrorType> {
	info!("Starting: Create account");

	// LOGIC

	AppResponse::builder()
		.body(LogoutResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
