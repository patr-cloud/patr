use axum::{http::StatusCode, response::IntoResponse};
use axum_typed_websockets::{Message, WebSocket};
use futures::StreamExt;
use models::{
	api::workspace::{
		deployment::StreamDeploymentShellServerMsg,
		runner::{
			StreamRunnerShellConnectionClientMsg,
			StreamRunnerShellConnectionPath,
			StreamRunnerShellConnectionRequest,
			StreamRunnerShellConnectionRequestHeaders,
			StreamRunnerShellConnectionServerMsg,
		},
	},
	utils::{GenericResponse, WebSocketUpgrade},
};
use rustis::{
	client::Client as RedisClient,
	commands::{ListCommands, StringCommands},
};
use tokio_util::sync::CancellationToken;

use crate::{
	models::shell_session::{
		SHELL_LIST_HIGH_WATER,
		SHELL_LIST_LOW_WATER,
		SHELL_POLL_MAX,
		SHELL_POLL_MIN,
		ShellSession,
		beacon_alive,
		pop_frame,
		push_frame,
		refresh_beacon,
	},
	prelude::*,
};

/// The websocket the runner dials back after being signalled to open a shell
/// session. Verifies the session is real and bound to *this* runner (the hijack
/// check), then bridges the runner's exec IO to the CLI-facing socket through
/// the same pair of bounded Redis Lists — applying backpressure toward the
/// container when the CLI side can't keep up.
pub async fn stream_runner_shell_connection(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					StreamRunnerShellConnectionPath {
						workspace_id,
						runner_id,
						session_id,
					},
				query: (),
				headers:
					StreamRunnerShellConnectionRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: WebSocketUpgrade(upgrade),
			},
		database: _,
		redis,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, StreamRunnerShellConnectionRequest>,
) -> Result<AppResponse<StreamRunnerShellConnectionRequest>, ErrorType> {
	// Load the session and enforce the binding *before* upgrading: a runner
	// authorised for its own `runner_id` must not be able to attach to a
	// session minted for a different runner, even if it learned the session id.
	let session_json: Option<String> = redis.get(redis::keys::shell_session(&session_id)).await?;
	let Some(session_json) = session_json else {
		// Expired handshake window or unknown session.
		return Err(ErrorType::ResourceDoesNotExist);
	};
	let session: ShellSession = serde_json::from_str(&session_json).map_err(|err| {
		error!("Corrupt shell session record for {session_id}: {err:?}");
		ErrorType::server_error("corrupt shell session record")
	})?;
	if session.runner_id != runner_id || session.workspace_id != workspace_id {
		warn!(
			"Runner {runner_id} tried to attach to shell session {session_id} bound to runner {}",
			session.runner_id
		);
		return Err(ErrorType::Unauthorized);
	}

	let redis = redis.clone();

	AppResponse::builder()
		.body(GenericResponse(
			upgrade
				.on_upgrade(move |websocket| async move {
					handle_runner_shell(websocket, session_id, redis).await;
				})
				.into_response(),
		))
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

/// Bridge the runner's exec websocket to the CLI-facing side: forward the
/// runner's output onto `to-client` (gated on its depth for backpressure),
/// drain `to-runner` down to the runner, and keep the liveness beacons fresh.
async fn handle_runner_shell(
	mut websocket: WebSocket<
		StreamRunnerShellConnectionServerMsg,
		StreamRunnerShellConnectionClientMsg,
	>,
	session_id: Uuid,
	redis: RedisClient,
) {
	use StreamRunnerShellConnectionClientMsg as FromRunner;
	use StreamRunnerShellConnectionServerMsg as ToRunner;

	let to_runner = redis::keys::shell_list_to_runner(&session_id);
	let to_client = redis::keys::shell_list_to_client(&session_id);
	let runner_alive = redis::keys::shell_runner_alive(&session_id);
	let session_key = redis::keys::shell_session(&session_id);

	// Signal the CLI side that the shell is live — this releases its bounded
	// dial-back wait.
	if push_frame(
		&redis,
		&to_client,
		&StreamDeploymentShellServerMsg::Connected,
	)
	.await
	.is_err()
	{
		let _ = websocket.close().await;
		return;
	}
	let _ = refresh_beacon(&redis, &runner_alive).await;

	let mut poll = SHELL_POLL_MIN;
	// Delay the first tick by one period — see the note in the CLI-facing
	// handler; `interval`'s immediate first tick would peer-check too eagerly.
	let mut beacon_timer = tokio::time::interval_at(
		tokio::time::Instant::now() + crate::models::shell_session::SHELL_BEACON_REFRESH,
		crate::models::shell_session::SHELL_BEACON_REFRESH,
	);
	// Backpressure: while `to-client` is above the high-water mark we stop
	// reading the runner's socket, which stalls the exec and (via TCP) the
	// container itself.
	let mut paused = false;

	loop {
		tokio::select! {
			_ = crate::GLOBAL_CANCEL_TOKEN
				.get_or_init(CancellationToken::new)
				.cancelled() =>
			{
				break;
			}
			incoming = websocket.next(), if !paused => {
				match incoming {
					// Runner disconnected mid-session.
					None => break,
					Some(Ok(Message::Item(from_runner))) => {
						let forwarded = match from_runner {
							FromRunner::Output { data } => {
								StreamDeploymentShellServerMsg::Output { data }
							}
							FromRunner::Exit { code } => {
								StreamDeploymentShellServerMsg::Exit { code }
							}
							FromRunner::Error { message } => {
								StreamDeploymentShellServerMsg::Error { message }
							}
						};
						let terminal = matches!(
							forwarded,
							StreamDeploymentShellServerMsg::Exit { .. }
								| StreamDeploymentShellServerMsg::Error { .. }
						);
						if push_frame(&redis, &to_client, &forwarded).await.is_err() {
							break;
						}
						if terminal {
							break;
						}
						// Re-check depth right after pushing so the gate reacts
						// promptly to a firehose.
						if let Ok(len) = redis.llen(&to_client).await
							&& len >= SHELL_LIST_HIGH_WATER
						{
							paused = true;
						}
					}
					Some(Ok(_)) => {}
					Some(Err(_)) => {}
				}
			}
			_ = tokio::time::sleep(poll) => {
				// While paused, watch for the queue to drain so we can resume.
				if paused
					&& let Ok(len) = redis.llen(&to_client).await
					&& len <= SHELL_LIST_LOW_WATER
				{
					paused = false;
				}
				match pop_frame::<ToRunner>(&redis, &to_runner).await {
					Ok(Some(frame)) => {
						let close = matches!(frame, ToRunner::Close);
						if websocket.send(Message::Item(frame)).await.is_err() {
							break;
						}
						poll = SHELL_POLL_MIN;
						if close {
							break;
						}
					}
					Ok(None) => {
						poll = (poll * 2).min(SHELL_POLL_MAX);
					}
					Err(err) => {
						error!("Redis error reading shell stdin: {err:?}");
						break;
					}
				}
			}
			_ = beacon_timer.tick() => {
				let _ = refresh_beacon(&redis, &runner_alive).await;
				// If the CLI-facing instance's beacon (the session key) lapsed,
				// it died — drop the runner so it kills the exec.
				if !beacon_alive(&redis, &session_key).await.unwrap_or(true) {
					break;
				}
			}
		}
	}

	// Teardown: push a fallback terminal frame in case the loop broke without one
	// (e.g. an abrupt runner disconnect), and tell the runner to close the exec.
	//
	// Deliberately do NOT `cleanup_session` here: this side is the *producer* of
	// `to-client`, and wiping it would delete the terminal `Exit`/`Error` before
	// the CLI-facing side (the consumer + session owner) has drained it — plus
	// deleting `shellRunnerAlive` would trip the CLI's "lost the runner" beacon
	// check. The CLI-facing side owns cleanup; the key TTLs are the backstop.
	let _ = push_frame(
		&redis,
		&to_client,
		&StreamDeploymentShellServerMsg::Exit { code: None },
	)
	.await;
	let _ = websocket.send(Message::Item(ToRunner::Close)).await;
	let _ = websocket.close().await;
}
