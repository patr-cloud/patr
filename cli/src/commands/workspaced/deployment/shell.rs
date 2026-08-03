use clap::Args as ClapArgs;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size};
use futures::{SinkExt, StreamExt};
use inquire::Select;
use models::api::{user::*, workspace::deployment::*};
use tokio::{
	io::{AsyncReadExt, AsyncWriteExt},
	sync::mpsc,
};

use crate::prelude::*;

/// The byte the CLI intercepts locally to force-disconnect a wedged session
/// (telnet's escape). Everything else is forwarded raw to the container.
const ESCAPE_BYTE: u8 = 0x1d;

/// Args for `patr deployment shell`.
#[derive(Debug, Clone, ClapArgs)]
pub struct Args {
	/// The name (or ID) of the deployment to open a shell into
	#[arg(
		short = 'n',
		long = "name",
		value_name = "NAME",
		env = "PATR_DEPLOYMENT_NAME"
	)]
	pub name: Option<String>,
}

/// Restores the terminal out of raw mode on every exit path, including panics.
struct RawModeGuard;

impl Drop for RawModeGuard {
	fn drop(&mut self) {
		let _ = disable_raw_mode();
	}
}

/// Open an interactive shell inside a running deployment (by name/id, or picked
/// interactively).
pub async fn execute(
	args: Args,
	global_args: GlobalArgs,
	state: AppState,
) -> Result<CommandOutput, AppError> {
	let AuthState::LoggedIn {
		token,
		current_workspace,
	} = state.auth
	else {
		return Err(AppError::NotLoggedIn);
	};

	let workspace_id = if let Some(workspace_id) = current_workspace {
		workspace_id
	} else {
		let workspaces = make_request(
			ApiRequest::<ListUserWorkspacesRequest>::builder()
				.headers(ListUserWorkspacesRequestHeaders {
					authorization: token.clone(),
					user_agent: constants::USER_AGENT,
				})
				.build(),
		)
		.await?
		.body
		.workspaces;

		let workspace_name = global_args.workspace.unwrap_or_else(|| {
			Select::new(
				"Please select a workspace to use",
				workspaces
					.iter()
					.map(|workspace| workspace.name.clone())
					.collect(),
			)
			.prompt()
			.expect_tty("Failed to read workspace ID")
		});

		workspaces
			.into_iter()
			.find(|workspace| {
				workspace.id.to_string() == workspace_name || workspace.name == workspace_name
			})
			.unwrap_or_else(|| panic!("No workspace found with ID or name: `{workspace_name}`"))
			.id
	};

	let mut deployments = vec![];
	let mut start = 0;

	loop {
		let response = make_request(
			ApiRequest::<ListDeploymentRequest>::builder()
				.path(ListDeploymentPath { workspace_id })
				.headers(ListDeploymentRequestHeaders {
					authorization: token.clone(),
					user_agent: constants::USER_AGENT,
				})
				.query(ListResourceQuery {
					page: start / ListResourceQuery::DEFAULT_PAGE_SIZE,
					count: ListResourceQuery::DEFAULT_PAGE_SIZE,
					search: Default::default(),
					sort: Default::default(),
					additional_query: (),
				})
				.build(),
		)
		.await?;

		start += response.body.deployments.len();

		deployments.extend(response.body.deployments);

		if start >= response.headers.total_count.0 {
			break;
		}
	}

	let deployment_id = args
		.name
		.and_then(|name| {
			let id = Uuid::parse_str(&name).ok();
			deployments
				.iter()
				.find(|r| r.name == name || id.filter(|id| r.id == *id).is_some())
				.map(|deployment| deployment.id)
		})
		.unwrap_or_else(|| {
			let name = Select::new(
				"Please select the deployment to open a shell into:",
				deployments
					.iter()
					.map(|deployment| &deployment.name)
					.collect(),
			)
			.prompt()
			.expect_tty("Failed to read deployment ID");

			deployments
				.iter()
				.find(|&deployment| &deployment.name == name)
				.unwrap_or_else(|| panic!("No deployment found with name: `{}`", name))
				.id
		});

	let stream = stream_request(
		ApiRequest::<StreamDeploymentShellRequest>::builder()
			.path(StreamDeploymentShellPath {
				workspace_id,
				deployment_id,
			})
			.headers(StreamDeploymentShellRequestHeaders {
				authorization: token.clone(),
				user_agent: constants::USER_AGENT,
			})
			.build(),
	)
	.await?;

	let (mut sink, mut read) = stream.split();

	// Connect phase: surface progress until the shell is live, all before raw
	// mode so Ctrl-C still cancels normally.
	loop {
		match read.next().await {
			Some(Ok(StreamDeploymentShellServerMsg::Connecting { message })) => {
				eprintln!("{message}...");
			}
			Some(Ok(StreamDeploymentShellServerMsg::Connected)) => break,
			Some(Ok(StreamDeploymentShellServerMsg::Error { message })) => {
				return Err(AppError::ApiError(ErrorType::server_error(message)));
			}
			Some(Ok(StreamDeploymentShellServerMsg::Exit { .. })) => {
				return Err(AppError::ApiError(ErrorType::server_error(
					"the shell session ended before it started",
				)));
			}
			Some(Ok(StreamDeploymentShellServerMsg::Output { .. })) => {}
			Some(Err(err)) => return Err(AppError::ApiError(err)),
			None => {
				return Err(AppError::ApiError(ErrorType::server_error(
					"the connection closed before the shell was ready",
				)));
			}
		}
	}

	eprintln!("Connected. Press Ctrl-] to disconnect.");

	// Raw mode: byte-transparent from here on. The guard restores the terminal
	// on any return or panic.
	enable_raw_mode().map_err(|err| AppError::ParseError(err.to_string()))?;
	let _guard = RawModeGuard;

	// All outbound frames funnel through one channel so stdin + resize share the
	// single sink.
	let (out_tx, mut out_rx) = mpsc::channel::<StreamDeploymentShellClientMsg>(256);

	// Initial size so full-screen programs render correctly from the start.
	if let Ok((cols, rows)) = size() {
		let _ = out_tx
			.send(StreamDeploymentShellClientMsg::Resize { rows, cols })
			.await;
	}

	let writer = tokio::spawn(async move {
		while let Some(msg) = out_rx.recv().await {
			if sink.send(msg).await.is_err() {
				break;
			}
		}
		let _ = sink.close().await;
	});

	// Fires when the user hits the escape byte (or stdin ends), so the output
	// loop can tear down.
	let (disconnect_tx, mut disconnect_rx) = tokio::sync::oneshot::channel::<()>();

	// stdin -> Stdin frames, watching for the local escape byte.
	let stdin_task = {
		let out_tx = out_tx.clone();
		tokio::spawn(async move {
			let mut stdin = tokio::io::stdin();
			let mut buf = [0u8; 4096];
			loop {
				match stdin.read(&mut buf).await {
					Ok(0) => break,
					Ok(n) => {
						let chunk = &buf[..n];
						// Forward everything up to a local escape byte, then bail.
						let (to_send, escape) = match chunk.iter().position(|&b| b == ESCAPE_BYTE) {
							Some(idx) => (&chunk[..idx], true),
							None => (chunk, false),
						};
						if !to_send.is_empty() &&
							out_tx
								.send(StreamDeploymentShellClientMsg::Stdin {
									data: to_send.into(),
								})
								.await
								.is_err()
						{
							break;
						}
						if escape {
							break;
						}
					}
					Err(_) => break,
				}
			}
			let _ = disconnect_tx.send(());
		})
	};

	// Terminal resize -> Resize frames (SIGWINCH; Unix only).
	#[cfg(unix)]
	let resize_task = {
		let out_tx = out_tx.clone();
		tokio::spawn(async move {
			use tokio::signal::unix::{SignalKind, signal};
			let Ok(mut winch) = signal(SignalKind::window_change()) else {
				return;
			};
			while winch.recv().await.is_some() {
				if let Ok((cols, rows)) = size() &&
					out_tx
						.send(StreamDeploymentShellClientMsg::Resize { rows, cols })
						.await
						.is_err()
				{
					break;
				}
			}
		})
	};

	// Output -> stdout, until the shell exits, errors, the escape byte fires, or
	// the connection drops.
	let mut stdout = tokio::io::stdout();
	let mut exit_code = 0;
	let mut error_message = None;
	let mut disconnected = false;

	loop {
		tokio::select! {
			frame = read.next() => {
				match frame {
					Some(Ok(StreamDeploymentShellServerMsg::Output { data })) => {
						let bytes: Vec<u8> = data.into();
						if stdout.write_all(&bytes).await.is_err() {
							break;
						}
						let _ = stdout.flush().await;
					}
					Some(Ok(StreamDeploymentShellServerMsg::Exit { code })) => {
						exit_code = code.unwrap_or(0);
						break;
					}
					Some(Ok(StreamDeploymentShellServerMsg::Error { message })) => {
						error_message = Some(message);
						break;
					}
					Some(Ok(_)) => {}
					Some(Err(_)) | None => {
						error_message = Some("lost connection to the deployment".to_owned());
						break;
					}
				}
			}
			// stdin hit the escape byte (or ended) — user-initiated disconnect.
			_ = &mut disconnect_rx => {
				disconnected = true;
				break;
			}
		}
	}

	// Teardown: stop the pumps and flush the terminal.
	stdin_task.abort();
	#[cfg(unix)]
	resize_task.abort();
	drop(out_tx);
	let _ = writer.await;
	let _ = stdout.flush().await;
	drop(_guard);

	if let Some(message) = error_message {
		eprintln!("\r\nShell session ended: {message}");
		std::process::exit(1);
	}
	if disconnected {
		eprintln!("\r\nDisconnected.");
	}
	std::process::exit(exit_code);
}
