use std::time::Duration;

use axum::{http::StatusCode, response::IntoResponse};
use axum_typed_websockets::Message;
use futures::{future::Either, prelude::stream::*};
use models::{
	api::workspace::runner::*,
	utils::{GenericResponse, WebSocketUpgrade},
};
use rustis::commands::{SetCondition, SetExpiration, StringCommands};

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
	// Try to acquire a lock on redis first
	let random_connection_id = Uuid::new_v4();
	let Ok(true) = redis
		.set_with_options(
			redis::keys::runner_connection_lock(&runner_id),
			random_connection_id.to_string(),
			SetCondition::NX,
			SetExpiration::Ex(
				const {
					if cfg!(debug_assertions) {
						5 // 5 seconds
					} else {
						120 // 2 mins
					}
				},
			),
			false,
		)
		.await
	else {
		return Err(ErrorType::RunnerAlreadyConnected);
	};

	let redis = redis.clone();

	AppResponse::builder()
		.body(GenericResponse(
			upgrade
				.on_upgrade(move |mut websocket| async move {
					let redis_channel = format!("{}/runner/{}/stream", workspace_id, runner_id);
					let mut pub_sub = redis.create_pub_sub();

					let Ok(()) = pub_sub
						.subscribe(&redis_channel)
						.await
						.inspect_err(|err| error!("Error streaming runner data: {:?}", err))
					else {
						return;
					};

					let ping_interval = if cfg!(debug_assertions) {
						Duration::from_secs(1)
					} else {
						Duration::from_secs(30)
					};

					let mut sleeper = Box::pin(tokio::time::sleep(ping_interval));
					let mut data_future = pub_sub.next();

					loop {
						match futures::future::select(sleeper, data_future).await {
							Either::Left((_, right)) => {
								data_future = right;
								sleeper = Box::pin(tokio::time::sleep(ping_interval));
								let Ok(_) = websocket.send(Message::Ping(Vec::new())).await else {
									debug!("Failed to send ping to websocket");
									break;
								};
								let Ok(true) = redis
									.set_with_options(
										redis::keys::runner_connection_lock(&runner_id),
										random_connection_id.to_string(),
										SetCondition::XX,
										SetExpiration::Ex(
											const {
												if cfg!(debug_assertions) {
													5 // 5 seconds
												} else {
													120 // 2 mins
												}
											},
										),
										false,
									)
									.await
								else {
									info!("Runner connection lock expired, closing websocket");
									break;
								};
							}
							Either::Right((data, left)) => {
								sleeper = left;
								data_future = pub_sub.next();
								let Some(Ok(data)) = data else {
									continue;
								};
								let Ok(data) =
									serde_json::from_slice(&data.payload).inspect_err(|err| {
										error!("Error streaming runner data: {:?}", err)
									})
								else {
									return;
								};
								debug!("Sending data down the pipe: {:#?}", data);
								let Ok(_) = websocket.send(Message::Item(data)).await else {
									debug!("Failed to send data to websocket");
									break;
								};
							}
						}
					}

					trace!("Websocket closed, unsubscribing from runner data stream");
					_ = pub_sub
						.unsubscribe(&redis_channel)
						.await
						.inspect_err(|err| error!("Error streaming runner data: {:?}", err));
				})
				.into_response(),
		))
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
