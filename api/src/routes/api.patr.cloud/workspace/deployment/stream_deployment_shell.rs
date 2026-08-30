use std::time::Instant;

use axum::{http::StatusCode, response::IntoResponse};
use axum_typed_websockets::{Message, WebSocket};
use futures::StreamExt;
use models::{
	api::workspace::{
		deployment::{
			DeploymentStatus,
			StreamDeploymentShellClientMsg,
			StreamDeploymentShellPath,
			StreamDeploymentShellRequest,
			StreamDeploymentShellRequestHeaders,
			StreamDeploymentShellServerMsg,
		},
		runner::{StreamRunnerDataForWorkspaceServerMsg, StreamRunnerShellConnectionServerMsg},
	},
	utils::{GenericResponse, WebSocketUpgrade},
};
use rustis::{
	client::Client as RedisClient,
	commands::{GenericCommands, PubSubCommands, StringCommands},
};
use tokio_util::sync::CancellationToken;

use crate::{
	models::shell_session::{
		SHELL_DIAL_BACK_TIMEOUT,
		SHELL_POLL_MAX,
		SHELL_POLL_MIN,
		SHELL_SESSION_TTL_SECS,
		ShellSession,
		cleanup_session,
		pop_frame,
		push_frame,
	},
	prelude::*,
};

/// Open an interactive shell inside a running deployment. Upgrades immediately
/// so every setup step (and every failure) is delivered to the CLI as a status
/// frame rather than an opaque hang, then coordinates with the runner over the
/// reverse-dial handshake and bridges stdin/stdout through a pair of bounded
/// Redis Lists (which may be relayed by a different API instance).
pub async fn stream_deployment_shell(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: StreamDeploymentShellPath {
					workspace_id,
					deployment_id,
				},
				query: (),
				headers:
					StreamDeploymentShellRequestHeaders {
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
	}: AuthenticatedAppRequest<'_, StreamDeploymentShellRequest>,
) -> Result<AppResponse<StreamDeploymentShellRequest>, ErrorType> {
	let redis = redis.clone();

	AppResponse::builder()
		.body(GenericResponse(
			upgrade
				.on_upgrade(move |websocket| async move {
					handle_client_shell(websocket, workspace_id, deployment_id, redis, state).await;
				})
				.into_response(),
		))
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

/// Send a terminal error frame to the CLI and give up. Best-effort — if the
/// socket is already gone there's nothing to report. The caller returns right
/// after, dropping (and thereby closing) the socket.
async fn fail(
	websocket: &mut WebSocket<StreamDeploymentShellServerMsg, StreamDeploymentShellClientMsg>,
	message: impl Into<String>,
) {
	let _ = websocket
		.send(Message::Item(StreamDeploymentShellServerMsg::Error {
			message: message.into(),
		}))
		.await;
}

/// Run the CLI-facing side of a shell session: the liveness/handshake steps
/// (with status frames), then the bridge loop relaying stdin/resize onto
/// `to-runner` and runner output off `to-client`.
async fn handle_client_shell(
	mut websocket: WebSocket<StreamDeploymentShellServerMsg, StreamDeploymentShellClientMsg>,
	workspace_id: Uuid,
	deployment_id: Uuid,
	redis: RedisClient,
	state: AppState,
) {
	use StreamDeploymentShellServerMsg as ServerMsg;

	// Step 1: resolve the runner + status for the deployment in one query.
	let progress = websocket
		.send(Message::Item(ServerMsg::Connecting {
			message: "Locating deployment".to_owned(),
		}))
		.await;
	if progress.is_err() {
		return;
	}

	let deployment = query!(
		r#"
		SELECT
			runner AS "runner: Uuid",
			status AS "status: DeploymentStatus"
		FROM
			deployment
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		deployment_id as _,
	)
	.fetch_optional(&state.database)
	.await;

	let deployment = match deployment {
		Ok(Some(row)) => row,
		Ok(None) => {
			fail(&mut websocket, "Deployment does not exist").await;
			return;
		}
		Err(err) => {
			error!("Failed to look up deployment {deployment_id}: {err:?}");
			fail(&mut websocket, "Internal error looking up the deployment").await;
			return;
		}
	};
	let runner_id = deployment.runner;

	// Step 2: the deployment has to actually be running to shell into it.
	if deployment.status != DeploymentStatus::Running {
		fail(
			&mut websocket,
			format!(
				"Deployment is not running (status: {:?})",
				deployment.status
			),
		)
		.await;
		return;
	}

	// Step 3: liveness pre-check — is the runner's control socket connected?
	// Reusing the same lock the control-socket handler maintains. Absent means
	// the runner is offline, so we fail fast instead of dialing into a void.
	let connection_lock: Option<String> = redis
		.get(redis::keys::runner_connection_lock(&runner_id))
		.await
		.unwrap_or(None);
	if connection_lock.is_none() {
		fail(
			&mut websocket,
			"The runner for this deployment is not connected",
		)
		.await;
		return;
	}

	// Step 4: mint the session (server-side only) and record the runner binding
	// used by the runner-facing hijack check.
	let session_id = Uuid::new_v4();
	let session = ShellSession {
		workspace_id,
		runner_id,
		deployment_id,
	};
	let session_key = redis::keys::shell_session(&session_id);
	let to_runner = redis::keys::shell_list_to_runner(&session_id);
	let to_client = redis::keys::shell_list_to_client(&session_id);
	let runner_alive = redis::keys::shell_runner_alive(&session_id);

	let session_json = serde_json::to_string(&session).expect("ShellSession serialises");
	if redis
		.setex(&session_key, SHELL_SESSION_TTL_SECS, session_json)
		.await
		.is_err()
	{
		fail(&mut websocket, "Internal error starting the shell session").await;
		return;
	}

	// Step 5: signal the runner over its control channel to dial back. Whichever
	// API instance holds that socket forwards this untouched.
	let _ = websocket
		.send(Message::Item(ServerMsg::Connecting {
			message: "Contacting runner".to_owned(),
		}))
		.await;
	let signal = StreamRunnerDataForWorkspaceServerMsg::ShellSessionRequested {
		session_id,
		deployment_id,
	};
	let signal_json = serde_json::to_string(&signal).expect("control signal serialises");
	if redis
		.publish(
			format!("{workspace_id}/runner/{runner_id}/stream"),
			signal_json,
		)
		.await
		.is_err()
	{
		fail(&mut websocket, "Failed to reach the runner").await;
		cleanup_session(&redis, &session_id).await;
		return;
	}

	// Step 6: bounded dial-back wait. The runner's first frame on `to-client`
	// is `Connected`; anything else is an early failure it wants to report.
	let deadline = Instant::now() + SHELL_DIAL_BACK_TIMEOUT;
	let first_frame = loop {
		match pop_frame::<ServerMsg>(&redis, &to_client).await {
			Ok(Some(frame)) => break Some(frame),
			Ok(None) => {
				if Instant::now() >= deadline {
					break None;
				}
				tokio::time::sleep(SHELL_POLL_MAX).await;
			}
			Err(err) => {
				error!("Redis error waiting for shell dial-back: {err:?}");
				break None;
			}
		}
	};
	match first_frame {
		Some(ServerMsg::Connected) => {
			if websocket
				.send(Message::Item(ServerMsg::Connected))
				.await
				.is_err()
			{
				cleanup_session(&redis, &session_id).await;
				return;
			}
		}
		Some(other) => {
			// Runner reported a failure before the shell came up — relay it.
			let _ = websocket.send(Message::Item(other)).await;
			let _ = websocket.close().await;
			cleanup_session(&redis, &session_id).await;
			return;
		}
		None => {
			fail(
				&mut websocket,
				"Timed out waiting for the runner to open the shell session",
			)
			.await;
			cleanup_session(&redis, &session_id).await;
			return;
		}
	}

	// Step 7: bridge. CLI stdin/resize -> `to-runner`; runner output (polled off
	// `to-client`) -> CLI. Refresh our liveness beacon and watch the runner's.
	let mut poll = SHELL_POLL_MIN;
	// Delay the first tick by one period — `interval`'s first tick fires
	// immediately, which would peer-check the runner's beacon before it has had
	// a chance to set it and tear the session down spuriously.
	let mut beacon_timer = tokio::time::interval_at(
		tokio::time::Instant::now() + crate::models::shell_session::SHELL_BEACON_REFRESH,
		crate::models::shell_session::SHELL_BEACON_REFRESH,
	);

	loop {
		tokio::select! {
			_ = crate::GLOBAL_CANCEL_TOKEN
				.get_or_init(CancellationToken::new)
				.cancelled() =>
			{
				break;
			}
			incoming = websocket.next() => {
				match incoming {
					// CLI closed the socket — client-initiated termination.
					None => break,
					Some(Ok(Message::Item(client_msg))) => {
						let forwarded = match client_msg {
							StreamDeploymentShellClientMsg::Stdin { data } => {
								StreamRunnerShellConnectionServerMsg::Stdin { data }
							}
							StreamDeploymentShellClientMsg::Resize { rows, cols } => {
								StreamRunnerShellConnectionServerMsg::Resize { rows, cols }
							}
						};
						if push_frame(&redis, &to_runner, &forwarded).await.is_err() {
							break;
						}
					}
					Some(Ok(_)) => {}
					Some(Err(_)) => {}
				}
			}
			_ = tokio::time::sleep(poll) => {
				match pop_frame::<ServerMsg>(&redis, &to_client).await {
					Ok(Some(frame)) => {
						let terminal = matches!(frame, ServerMsg::Exit { .. } | ServerMsg::Error { .. });
						if websocket.send(Message::Item(frame)).await.is_err() {
							break;
						}
						poll = SHELL_POLL_MIN;
						if terminal {
							break;
						}
					}
					Ok(None) => {
						poll = (poll * 2).min(SHELL_POLL_MAX);
					}
					Err(err) => {
						error!("Redis error reading shell output: {err:?}");
						break;
					}
				}
			}
			_ = beacon_timer.tick() => {
				// Extend the session key's TTL (our beacon) without clobbering
				// the JSON record.
				let _ = redis
					.expire(&session_key, SHELL_SESSION_TTL_SECS, None)
					.await;
				// If the runner-facing instance's beacon lapsed, it died — bail.
				let alive: usize = redis.exists(&runner_alive).await.unwrap_or(1);
				if alive == 0 {
					let _ = websocket
						.send(Message::Item(ServerMsg::Error {
							message: "Lost connection to the runner".to_owned(),
						}))
						.await;
					break;
				}
			}
		}
	}

	// Teardown: tell the runner side to kill the exec, wipe session state, close.
	let _ = push_frame(
		&redis,
		&to_runner,
		&StreamRunnerShellConnectionServerMsg::Close,
	)
	.await;
	cleanup_session(&redis, &session_id).await;
	let _ = websocket.close().await;
}
