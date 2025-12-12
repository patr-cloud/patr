use std::time::Duration;

use axum::{http::StatusCode, response::IntoResponse};
use axum_typed_websockets::{Message, WebSocket};
use futures::{
	future::{self, Either},
	prelude::stream::*,
};
use models::{
	api::workspace::{
		deployment::DeploymentStatus,
		runner::{StreamRunnerDataForWorkspaceClientMsg::*, *},
	},
	utils::{GenericResponse, WebSocketUpgrade},
};
use rustis::{
	client::Client as RedisClient,
	commands::{SetCondition, SetExpiration, StringCommands},
};
use tokio_util::sync::CancellationToken;

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
		user_data: _,
		state,
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
				.on_upgrade(async move |websocket| {
					handle_websocket(
						websocket,
						workspace_id,
						runner_id,
						redis,
						random_connection_id,
						state.database,
					)
					.await
				})
				.into_response(),
		))
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

async fn handle_websocket(
	mut websocket: WebSocket<
		StreamRunnerDataForWorkspaceServerMsg,
		StreamRunnerDataForWorkspaceClientMsg,
	>,
	workspace_id: Uuid,
	runner_id: Uuid,
	redis: RedisClient,
	random_connection_id: Uuid,
	database: sqlx::Pool<DatabaseType>,
) {
	let redis_channel = format!("{workspace_id}/runner/{runner_id}/stream");
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

	loop {
		// Make sure to check for cancellation
		let Some(actionable_future) = future::select(
			future::select(
				future::select(pub_sub.next(), websocket.next()),
				&mut sleeper,
			),
			std::pin::pin!(
				crate::GLOBAL_CANCEL_TOKEN
					.get_or_init(CancellationToken::new)
					.cancelled()
			),
		)
		.await
		.into_left() else {
			// Global cancellation triggered
			debug!("Global cancellation triggered, closing websocket");
			break;
		};

		let Some(reader_writer) = actionable_future.into_left() else {
			// Reset the sleeper for the next ping interval
			sleeper = Box::pin(tokio::time::sleep(ping_interval));

			let Ok(_) = websocket.send(Message::Ping(Default::default())).await else {
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
			continue;
		};

		match reader_writer {
			Either::Left((publish_data, _)) => {
				let Some(data) = publish_data else {
					// pub_sub stream ended
					debug!("Redis pub/sub stream ended");
					break;
				};
				let Ok(data) = data else {
					// Error on pub_sub, continue to try again
					continue;
				};
				let Ok(data) = serde_json::from_slice(&data.payload)
					.inspect_err(|err| error!("Error streaming runner data: {:?}", err))
				else {
					break;
				};
				trace!("Sending data down the pipe: {:?}", data);
				let Ok(_) = websocket.send(Message::Item(data)).await else {
					debug!("Failed to send data to websocket");
					break;
				};
			}
			Either::Right((client_message, _)) => {
				let Some(message) = client_message else {
					// Websocket stream ended (client disconnected)
					debug!("Websocket client disconnected");
					break;
				};
				let Ok(message) = message else {
					// Error on websocket, continue to try again
					continue;
				};
				let Message::Item(message) = message else {
					continue;
				};
				trace!("Received message from websocket: {:?}", message);

				match message {
					DeploymentStatusUpdated { id, status } => {
						let Ok(()) = update_deployment_status(id, status, &database)
							.await
							.inspect_err(|err| {
								error!(
									"Failed to update deployment status for deployment ID: {}: {:?}",
									id, err
								);
							})
						else {
							error!("Failed to update deployment status for deployment ID: {id}");
							continue;
						};
					}
				}
			}
		}
	}

	trace!("Websocket closed, unsubscribing from runner data stream");
	_ = pub_sub
		.unsubscribe(&redis_channel)
		.await
		.inspect_err(|err| error!("Error streaming runner data: {:?}", err));
	_ = websocket.close().await;
}

async fn update_deployment_status(
	id: Uuid,
	status: DeploymentStatus,
	database: &sqlx::Pool<DatabaseType>,
) -> Result<(), ErrorType> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			status = $1
		WHERE
			id = $2 AND
			status != $1 AND (
				(
					$1 = 'errored' AND status = 'deploying'
				) OR (
					$1 = 'deploying' AND status = 'errored'
				) OR (
					$1 IN ('deploying', 'errored') AND status = 'running'
				) OR (
				 	$1 = 'running' AND status IN ('deploying', 'errored')
				)
			)
		RETURNING
			status AS "status: DeploymentStatus";
		"#,
		status as _,
		id as _,
	)
	.fetch_one(database)
	.await?;

	Ok(())
}
