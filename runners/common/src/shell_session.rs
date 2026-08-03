//! The runner side of the interactive deployment-shell feature.
//!
//! When the control socket delivers a `ShellSessionRequested`, the WebSocket
//! actor spawns [`run_shell_session`] as a detached task. It dials a dedicated
//! per-session websocket back to the API and pumps bytes between that socket
//! and the executor's exec (via [`ShellIo`]) until either side ends. It is
//! deliberately *not* a `ractor` actor: nothing sends it messages, it must not
//! restart, and it must be unlinked from the control socket so a shell ending
//! never tears the control connection down.

use futures::{SinkExt, StreamExt};
use models::api::workspace::runner::{
	StreamRunnerShellConnectionClientMsg,
	StreamRunnerShellConnectionPath,
	StreamRunnerShellConnectionRequest,
	StreamRunnerShellConnectionRequestHeaders,
	StreamRunnerShellConnectionServerMsg,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::prelude::*;

/// Bounded capacity of the runner's shell IO channels. Small on purpose: it's
/// what makes `outbound` apply backpressure to the exec (and thereby the
/// container) when the far end can't keep up.
const SHELL_CHANNEL_CAPACITY: usize = 128;

/// Input flowing from the user's terminal toward the exec.
pub enum ShellInput {
	/// Raw stdin bytes.
	Data(Vec<u8>),
	/// The user's terminal was resized.
	Resize {
		/// Number of rows.
		rows: u16,
		/// Number of columns.
		cols: u16,
	},
}

/// The IO handles handed to a [`RunnerExecutor::open_deployment_shell`] impl.
/// Keeping this bollard-free (plain channels) is what lets `runners/common`
/// stay generic over the executor without depending on Docker.
pub struct ShellIo {
	/// Stdin + resize events from the user. Yields `None` when the session
	/// ends — the executor should finish when it does.
	pub inbound: mpsc::Receiver<ShellInput>,
	/// The container's output back to the user. Bounded, so a fast-printing
	/// process blocks on `send` (backpressure) instead of dropping bytes.
	pub outbound: mpsc::Sender<Vec<u8>>,
}

/// Drive one interactive shell session end to end. Spawned detached; returns
/// when the exec exits, the session closes, or the runner is shutting down.
pub async fn run_shell_session<E: RunnerExecutor>(
	config: RunnerSettings<E::Settings>,
	session_id: Uuid,
	deployment_id: Uuid,
	executor: E,
) {
	let RunnerMode::Managed {
		workspace_id,
		runner_id,
		api_token,
		user_agent,
	} = config.mode.clone()
	else {
		// Only managed runners receive shell-session requests.
		return;
	};

	let stream = match client::stream_request(
		ApiRequest::<StreamRunnerShellConnectionRequest>::builder()
			.path(StreamRunnerShellConnectionPath {
				workspace_id,
				runner_id,
				session_id,
			})
			.headers(StreamRunnerShellConnectionRequestHeaders {
				authorization: api_token,
				user_agent,
			})
			.build(),
	)
	.await
	{
		Ok(stream) => stream,
		Err(err) => {
			error!("Failed to dial shell session {session_id}: {err:?}");
			return;
		}
	};

	let (mut sink, mut read) = stream.split();

	let (in_tx, in_rx) = mpsc::channel::<ShellInput>(SHELL_CHANNEL_CAPACITY);
	let (out_tx, mut out_rx) = mpsc::channel::<Vec<u8>>(SHELL_CHANNEL_CAPACITY);
	// One channel for everything we send back to the API, so the output pump
	// and the terminal Exit/Error frame share the single sink.
	let (ws_out_tx, mut ws_out_rx) =
		mpsc::channel::<StreamRunnerShellConnectionClientMsg>(SHELL_CHANNEL_CAPACITY);

	// Writer: drain outgoing frames onto the websocket.
	let writer = tokio::spawn(async move {
		while let Some(msg) = ws_out_rx.recv().await {
			if sink.send(msg).await.is_err() {
				break;
			}
		}
		let _ = sink.close().await;
	});

	// Output pump: exec output -> `Output` frames. `Base64String` handles the
	// wire encoding, so we deal in raw bytes here. Ends when `out_rx` closes
	// (i.e. when the executor drops `ShellIo::outbound`).
	let out_pump = {
		let ws_out_tx = ws_out_tx.clone();
		tokio::spawn(async move {
			while let Some(bytes) = out_rx.recv().await {
				if ws_out_tx
					.send(StreamRunnerShellConnectionClientMsg::Output { data: bytes.into() })
					.await
					.is_err()
				{
					break;
				}
			}
		})
	};

	// Reader: API frames -> `ShellInput`. Dropping `in_tx` on close/end signals
	// the executor to finish.
	let reader = tokio::spawn(async move {
		while let Some(item) = read.next().await {
			match item {
				Ok(StreamRunnerShellConnectionServerMsg::Stdin { data }) => {
					let bytes: Vec<u8> = data.into();
					if in_tx.send(ShellInput::Data(bytes)).await.is_err() {
						break;
					}
				}
				Ok(StreamRunnerShellConnectionServerMsg::Resize { rows, cols }) => {
					let _ = in_tx.send(ShellInput::Resize { rows, cols }).await;
				}
				Ok(StreamRunnerShellConnectionServerMsg::Close) | Err(_) => break,
			}
		}
	});

	let io = ShellIo {
		inbound: in_rx,
		outbound: out_tx,
	};

	// Run the exec until it finishes, the session closes, or the runner shuts
	// down.
	let exec_result = tokio::select! {
		result = executor.open_deployment_shell(deployment_id, io) => result,
		_ = crate::runner::GLOBAL_CANCEL_TOKEN
			.get_or_init(CancellationToken::new)
			.cancelled() =>
		{
			Ok(0)
		}
	};

	// Report the outcome so the CLI is never left hanging.
	let terminal = match exec_result {
		Ok(code) => StreamRunnerShellConnectionClientMsg::Exit { code: Some(code) },
		Err(err) => StreamRunnerShellConnectionClientMsg::Error {
			message: err.to_string(),
		},
	};
	let _ = ws_out_tx.send(terminal).await;

	// Teardown: dropping the last sender lets the writer flush and close; the
	// reader/pump are no longer needed.
	drop(ws_out_tx);
	reader.abort();
	out_pump.abort();
	let _ = writer.await;
}
