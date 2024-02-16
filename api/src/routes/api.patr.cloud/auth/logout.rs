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
					user_agent,
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
	info!("Recieved logout request");

	// LOGIC

	AppResponse::builder()
		.body(LogoutResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
