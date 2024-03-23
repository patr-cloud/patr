use axum::{http::StatusCode, response::IntoResponse, Router};
use models::{api::workspace::runner::*, utils::GenericResponse};

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new().mount_auth_endpoint(stream_runner_data_for_workspace, state)
}

async fn stream_runner_data_for_workspace(
	AuthenticatedAppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, StreamRunnerDataForWorkspaceRequest>,
) -> Result<AppResponse<StreamRunnerDataForWorkspaceRequest>, ErrorType> {
	AppResponse::builder()
		.body(GenericResponse(
			body.0.on_upgrade(|websocket| async move {}).into_response(),
		))
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
