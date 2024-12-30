use std::{net::SocketAddr, pin::pin};

use dashmap::DashMap;
use futures::{
	future::{self, Either},
	stream::BoxStream,
	StreamExt,
};
use models::{api::workspace::runner::*, rbac::ResourceType};
use tokio::{
	net::TcpListener,
	sync::mpsc::unbounded_channel,
	task,
	time::{self, Duration},
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{level_filters::LevelFilter, Dispatch, Level};
use tracing_subscriber::{
	fmt::{format::FmtSpan, Layer as FmtLayer},
	layer::SubscriberExt,
	Layer,
};

use crate::{db, prelude::*, utils::resource_executor::ResourceExecutorTask};

/// All deployment related functions for the runner
mod deployment;

/// The runner is the main struct that is used to run the resources.
///
/// It contains the executor, the database connection pool, and the settings for
/// the runner. The runner is created using the [`Runner::new`] function.
pub struct Runner<E>
where
	E: RunnerExecutor,
{
	/// Runner task registry
	registry: DashMap<Uuid, ResourceExecutorTask>,
	/// State and configuration for the runner
	state: AppState<E>,
}

impl<E> Runner<E>
where
	E: RunnerExecutor + Clone + 'static,
{
	/// Initializes and runs the runner. This function will create a new
	/// database connection pool and set up the global default subscriber for
	/// the runner, then start the runner and run the resources that the runner
	/// is responsible for. This function will run forever until the runner is
	/// stopped.
	pub async fn run() {
		let config = RunnerSettings::<E::Settings>::parse(E::RUNNER_INTERNAL_NAME)
			.expect("Failed to parse settings");

		tracing::dispatcher::set_global_default(Dispatch::new(
			tracing_subscriber::registry().with(
				FmtLayer::new()
					.with_span_events(FmtSpan::NONE)
					.event_format(
						tracing_subscriber::fmt::format()
							.with_ansi(true)
							.with_file(false)
							.without_time()
							.compact(),
					)
					.with_filter(
						tracing_subscriber::filter::Targets::new()
							.with_target(E::RUNNER_INTERNAL_NAME, LevelFilter::TRACE)
							.with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE)
							.with_target("models", LevelFilter::TRACE),
					)
					.with_filter(LevelFilter::from_level(
						if config.environment == RunningEnvironment::Development {
							Level::TRACE
						} else {
							Level::DEBUG
						},
					)),
			),
		))
		.expect("Failed to set global default subscriber");

		let database = db::connect(&config.database).await;

		let (runner_changes_sender, runner_changes_receiver) = unbounded_channel();
		let mut runner_changes_receiver = UnboundedReceiverStream::new(runner_changes_receiver);

		let mut runner = Self {
			registry: DashMap::new(),
			state: AppState {
				database,
				runner_changes_sender,
				config,
			},
		};

		let state = runner.state.clone();
		// Run the server here
		let server_task = task::spawn(async move {
			let tcp_listener = TcpListener::bind(state.config.bind_address).await.unwrap();

			info!(
				"Listening for connections on http://{}",
				tcp_listener.local_addr().unwrap()
			);

			axum::serve(
				tcp_listener,
				crate::routes::setup_routes(&state)
					.await
					.into_make_service_with_connect_info::<SocketAddr>(),
			)
			.with_graceful_shutdown(exit_signal())
			.await
			.unwrap();
		});

		info!("Runner started");

		let mut exit_signal = pin!(exit_signal());
		debug!("Exit signal listener started");

		info!("Connecting to the server");
		// Connect to the server infinitely until the exit signal is received
		'main: loop {
			// Initialize the runner changes receiver
			let Some(response) = futures::future::select(
				&mut exit_signal,
				pin!(runner.get_update_resources_stream(&mut runner_changes_receiver)),
			)
			.await
			.into_right() else {
				// Left branch is the exit signal
				warn!("Exit signal received. Stopping runner");
				break 'main;
			};
			info!("Connected to the server");

			let Ok(stream) = response
				.inspect_err(|err| {
					error!("Failed to connect to the server: {:?}", err);
					error!("Retrying in 5 second");
				})
				.map_err(|err| err.body)
			else {
				// Retry after 5 seconds, but break if the exit signal is received
				if future::select(&mut exit_signal, pin!(time::sleep(Duration::from_secs(5))))
					.await
					.is_left()
				{
					// Left branch is the exit signal
					break 'main;
				};
				continue 'main;
			};

			info!("Reconciling all resources before starting");
			// Reconcile all resources at the start (or when reconnecting to the websocket)
			runner.reconcile_all().await;

			// Reconcile all resources at a fixed interval
			let mut reconcile_all = Box::pin(time::sleep(E::FULL_RECONCILIATION_INTERVAL));
			let mut pinned_stream = pin!(stream);

			'message: loop {
				let Some(reconcile_all_or_one) = future::select(
					&mut exit_signal,
					future::select(reconcile_all.as_mut(), pinned_stream.next()),
				)
				.await
				.into_right() else {
					// Left branch is the exit signal
					break 'main;
				};

				let reconcile_message = match reconcile_all_or_one {
					// Reconcile all resources
					Either::Left(_) => {
						reconcile_all = Box::pin(time::sleep(E::FULL_RECONCILIATION_INTERVAL));
						runner.reconcile_all().await;
						continue 'message;
					}
					Either::Right((actionable_message, _)) => actionable_message,
				};

				match reconcile_message {
					Some(Ok(response)) => {
						runner.handle_server_message(response).await;
					}
					Some(Err(err)) => {
						// Data from the websocket failed
						error!("Failed to connect to the server: {:?}", err);
						error!("Retrying in 1 second");
						// Retry after 5 seconds, but break if the exit signal is received
						if future::select(
							&mut exit_signal,
							pin!(time::sleep(Duration::from_secs(1))),
						)
						.await
						.is_right()
						{
							// Left branch is the exit signal
							break 'main;
						};

						continue 'main;
					}
					None => {
						// Websocket disconnected. Reconnect
						error!("Connection to server closed");
						error!("Retrying in 2 seconds");
						// Retry after 5 seconds, but break if the exit signal is received
						if future::select(
							&mut exit_signal,
							pin!(time::sleep(Duration::from_secs(2))),
						)
						.await
						.is_right()
						{
							// Left branch is the exit signal
							break 'main;
						};

						continue 'main;
					}
				}
			}
		}

		info!("Runner stopped. Waiting for server to exit");
		_ = server_task.await;
		info!("Server exited. Exiting runner...");
	}

	/// Reconcile all the resources that the runner is responsible for. This
	/// function will run the reconciliation for all the resources that the
	/// runner is responsible for.
	async fn reconcile_all(&mut self) {
		// Reconcile all resources
		self.reconcile_all_deployments().await;
	}

	/// Handle a message from the server. This function will handle the message
	/// from the server and run the reconciliation for the resource that the
	/// message is for.
	async fn handle_server_message(&mut self, msg: StreamRunnerDataForWorkspaceServerMsg) {
		info!("Handling server message: {:?}", msg);
		// if this resource is already queued for reconciliation, remove that
		let resource_id = get_resource_id_from_message(&msg);

		match msg.resource_type() {
			ResourceType::Deployment => {
				self.reconcile_deployment(resource_id).await;
			}
			_ => {
				warn!("Unknown resource type: {:?}", msg);
			}
		}
	}

	/// Get the stream of updates for the runner.
	///
	/// If the runner is running in self-hosted mode, this function will return
	/// the stream of updates from the runner changes receiver. If the runner is
	/// running in managed mode, this function will return the stream of updates
	/// from the websocket endpoint to the Patr API.
	async fn get_update_resources_stream<'a>(
		&mut self,
		runner_changes_receiver: &'a mut UnboundedReceiverStream<
			StreamRunnerDataForWorkspaceServerMsg,
		>,
	) -> Result<
		BoxStream<'a, Result<StreamRunnerDataForWorkspaceServerMsg, ErrorType>>,
		ApiErrorResponse,
	> {
		match &self.state.config.mode {
			RunnerMode::SelfHosted {
				password_pepper: _,
				jwt_secret: _,
			} => Ok(runner_changes_receiver
				.map(|msg| Ok::<_, ErrorType>(msg))
				.boxed()),
			RunnerMode::Managed {
				workspace_id,
				runner_id,
				api_token,
				user_agent,
			} => client::stream_request(
				ApiRequest::<StreamRunnerDataForWorkspaceRequest>::builder()
					.path(StreamRunnerDataForWorkspacePath {
						workspace_id: *workspace_id,
						runner_id: *runner_id,
					})
					.headers(StreamRunnerDataForWorkspaceRequestHeaders {
						authorization: api_token.clone(),
						user_agent: user_agent.clone(),
					})
					.query(())
					.body(Default::default())
					.build(),
			)
			.await
			.map(StreamExt::boxed),
		}
	}
}

/// Listen for the exit signal and stop the runner when the signal is received.
#[tracing::instrument]
async fn exit_signal() {
	let ctrl_c = async {
		tokio::signal::ctrl_c()
			.await
			.expect("Failed to listen for SIGINT")
	};

	#[cfg(unix)]
	let terminate = async {
		tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
			.expect("failed to install signal handler")
			.recv()
			.await;
	};

	#[cfg(not(unix))]
	let terminate = std::future::pending::<()>();

	tokio::select! {
		_ = ctrl_c => (),
		_ = terminate => (),
	}
	info!("Shutdown signal received, shutting down server gracefully");
}

/// For a given message, get the resource ID from the message
fn get_resource_id_from_message(message: &StreamRunnerDataForWorkspaceServerMsg) -> Uuid {
	use StreamRunnerDataForWorkspaceServerMsg::*;
	match message {
		DeploymentCreated { deployment, .. } => deployment.id,
		DeploymentUpdated { deployment, .. } => deployment.id,
		DeploymentDeleted { id } => *id,
	}
}
