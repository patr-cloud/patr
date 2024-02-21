use axum::http::StatusCode;
use models::api::auth::{LogoutPath, LogoutRequest, LogoutRequestHeaders, LogoutResponse};

use crate::prelude::*;

pub async fn logout(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: LogoutPath,
				query: (),
				headers: LogoutRequestHeaders {
					refresh_token,
					user_agent: _,
				},
				body,
			},
		database,
		redis: _,
		client_ip: _,
		user_data,
		config,
	}: AuthenticatedAppRequest<'_, LogoutRequest>,
) -> Result<AppResponse<LogoutRequest>, ErrorType> {
	info!("Logging out user: {}", user_data.id);

	// User agent being a browser is expected to be checked in the
	// UserAgentValidationLayer

	// LOGIC

	AppResponse::builder()
		.body(LogoutResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
