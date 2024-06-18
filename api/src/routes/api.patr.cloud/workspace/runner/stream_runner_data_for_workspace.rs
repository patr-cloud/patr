use std::time::Duration;

use axum::{http::StatusCode, response::IntoResponse};
use axum_typed_websockets::Message;
use futures::prelude::stream::*;
use models::{
	api::workspace::runner::*,
	utils::{GenericResponse, WebSocketUpgrade},
};

use crate::prelude::*;

pub async fn stream_runner_data_for_workspace(
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
				body: WebSocketUpgrade(upgrade),
			},
		database: _,
		redis,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, StreamRunnerDataForWorkspaceRequest>,
) -> Result<AppResponse<StreamRunnerDataForWorkspaceRequest>, ErrorType> {
	let redis = redis.clone();

	AppResponse::builder()
		.body(GenericResponse(
			upgrade
				.on_upgrade(move |mut websocket| async move {
					let redis_channel = format!("{}/runner/{}/stream", workspace_id, runner_id);
					let result: Result<(), Box<dyn std::error::Error>> = try {
						let mut pub_sub = redis.create_pub_sub();

						pub_sub.subscribe(&redis_channel).await?;

						let ping_interval = if cfg!(debug_assertions) {
							Duration::from_secs(1)
						} else {
							Duration::from_secs(30)
						};

						loop {
							let Ok(data) = pub_sub.next().timeout(ping_interval).await else {
								let Ok(_) = websocket.send(Message::Ping(Vec::new())).await else {
									break;
								};
								continue;
							};

							if let Some(Ok(data)) = data {
								let data = serde_json::from_slice::<
									StreamRunnerDataForWorkspaceServerMsg,
								>(&data.payload)?;
								let Ok(_) = websocket.send(Message::Item(data)).await else {
									break;
								};
							}
						}

						trace!("Websocket closed, unsubscribing from runner data stream");
						pub_sub.unsubscribe(&redis_channel).await?;
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
