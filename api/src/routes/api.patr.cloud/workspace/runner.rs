use axum::{http::StatusCode, response::IntoResponse, Router};
use axum_typed_websockets::Message;
use futures::StreamExt;
use models::{api::workspace::runner::*, utils::GenericResponse};

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new().mount_auth_endpoint(stream_runner_data_for_workspace, state)
}

async fn stream_runner_data_for_workspace(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: StreamRunnerDataForWorkspacePath {
					workspace_id,
					runner_id,
				},
				query: (),
				headers:
					StreamRunnerDataForWorkspaceRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body,
			},
		database,
		redis,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, StreamRunnerDataForWorkspaceRequest>,
) -> Result<AppResponse<StreamRunnerDataForWorkspaceRequest>, ErrorType> {
	let redis = redis.clone();

	AppResponse::builder()
		.body(GenericResponse(
			body.0
				.on_upgrade(move |mut websocket| async move {
					let result: Result<(), Box<dyn std::error::Error>> = try {
						let mut pub_sub = redis.create_pub_sub();

						pub_sub
							.subscribe(format!("{}/runner/{}/stream", workspace_id, runner_id))
							.await?;

						while let Some(Ok(data)) = pub_sub.next().await {
							let data = serde_json::from_slice::<
								StreamRunnerDataForWorkspaceServerMsg,
							>(&data.payload)?;
							websocket.send(Message::Item(data)).await?;
						}
					};

					if let Err(e) = result {
						error!("Error streaming runner data: {:?}", e);
					}
				})
				.into_response(),
		))
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
