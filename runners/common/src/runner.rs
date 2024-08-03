use std::{collections::HashMap, future::Future, pin::pin, str::FromStr};

use futures::{
	future::{self, Either},
	FutureExt,
	StreamExt,
};
use models::api::workspace::{deployment::*, runner::*};
use tokio::time::{self, Duration, Instant};
use tracing::{level_filters::LevelFilter, Dispatch, Level};
use tracing_subscriber::{
	fmt::{format::FmtSpan, Layer as FmtLayer},
	layer::SubscriberExt,
	Layer,
};

use crate::prelude::*;

/// The runner is the main struct that is used to run the resources. It contains
/// the executor, the database connection pool, and the settings for the runner.
/// The runner is created using the [`Runner::new`] function.
pub struct Runner<'de, E>
where
	E: RunnerExecutor,
{
	/// The executor for the runner. This is the main trait that the runner
	/// needs to implement to run the resources.
	executor: E,
	/// The settings for the runner. This contains the workspace ID, the runner
	/// ID, the API token, the environment, and any additional settings that the
	/// runner needs.
	settings: RunnerSettings<E::Settings<'de>>,
	/// The state of the runner. This is used to store the list of deployments,
	/// databases, etc. that the runner is running.
	deployments: HashMap<Uuid, (Deployment, DeploymentRunningDetails)>,
}

impl<'de, E> Runner<'de, E>
where
	E: RunnerExecutor,
{
	/// Create a new runner with the given executor. This function will create a
	/// new database connection pool and set up the global default subscriber
	/// for the runner.
	pub async fn new(executor: E) -> Self {
		let settings = get_runner_settings();

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
							.with_target(env!("CARGO_PKG_NAME"), LevelFilter::TRACE)
							.with_target("models", LevelFilter::TRACE),
					)
					.with_filter(LevelFilter::from_level(
						if settings.environment == RunningEnvironment::Development {
							Level::TRACE
						} else {
							Level::DEBUG
						},
					)),
			),
		))
		.expect("Failed to set global default subscriber");

		let deployments = HashMap::new();

		Self {
			executor,
			settings,
			deployments,
		}
	}

	/// Run the runner. This function will start the runner and run the
	/// resources that the runner is responsible for. This function will run
	/// forever until the runner is stopped.
	pub async fn run(mut self) -> Result<(), ErrorType> {
		let authorization = BearerToken::from_str(&self.settings.api_token)?;
		let user_agent = UserAgent::from_str(concat!(
			env!("CARGO_PKG_NAME"),
			"/",
			env!("CARGO_PKG_VERSION"),
		))?;
		let workspace_id = self.settings.workspace_id;
		let runner_id = self.settings.runner_id;

		let mut exit_signal = pin!(exit_signal());

		// Connect to the server infinitely until the exit signal is received
		'main: loop {
			let Some(response) = futures::future::select(
				&mut exit_signal,
				Box::pin(crate::client::stream_request(
					ApiRequest::<StreamRunnerDataForWorkspaceRequest>::builder()
						.path(StreamRunnerDataForWorkspacePath {
							workspace_id,
							runner_id,
						})
						.headers(StreamRunnerDataForWorkspaceRequestHeaders {
							authorization: authorization.clone(),
							user_agent: user_agent.clone(),
						})
						.query(())
						.body(Default::default())
						.build(),
				)),
			)
			.await
			.into_right() else {
				// Left branch is the exit signal
				break 'main;
			};

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

			// Reconcile all resources at a fixed interval
			let mut reconcile_all = Box::pin(time::sleep(E::FULL_RECONCILIATION_INTERVAL));
			// If any resource failes to be reconciled, it will be requeued here
			let reconcile_requeues = Vec::<(StreamRunnerDataForWorkspaceServerMsg, Instant)>::new();
			let mut next_reconcile_future = get_next_reconcile_future(&reconcile_requeues);
			let mut pinned_stream = pin!(stream);

			// Reconcile all resources at the start (or when reconnecting to the websocket)
			self.reconcile_all().await;

			'message: loop {
				let Some(reconcile_all_or_one) = future::select(
					&mut exit_signal,
					future::select(
						reconcile_all.as_mut(),
						future::select(pinned_stream.next(), &mut next_reconcile_future),
					),
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
						self.reconcile_all().await;
						continue 'message;
					}
					Either::Right((actionable_message, _)) => actionable_message,
				};

				match reconcile_message {
					// Reconcile a resource from the server
					Either::Left((Some(Ok(response)), _)) => {
						self.reconcile(response).await;
					}
					// Data from the websocket failed
					Either::Left((Some(Err(err)), _)) => {
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
					// Websocket disconnected. Reconnect
					Either::Left((None, _)) => {
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
					// A specific resource needs to be reconciled again
					Either::Right((response, _)) => {
						self.reconcile(response).await;
						next_reconcile_future = get_next_reconcile_future(&reconcile_requeues);
					}
				}
			}
		}

		info!("Runner stopped. Exiting...");
		Ok(())
	}

	/// Reconcile all the resources that the runner is responsible for. This
	/// function will run the reconciliation for all the resources that the
	/// runner is responsible for.
	async fn reconcile_all(&mut self) {}

	async fn reconcile(&mut self, _: StreamRunnerDataForWorkspaceServerMsg) {}
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

fn get_next_reconcile_future(
	reconcile_requeues: &[(StreamRunnerDataForWorkspaceServerMsg, Instant)],
) -> std::pin::Pin<Box<dyn Future<Output = StreamRunnerDataForWorkspaceServerMsg> + Send>> {
	reconcile_requeues
		.iter()
		.reduce(|a, b| if a.1 < b.1 { a } else { b })
		.map(|(message, instant)| {
			let message = message.clone();
			time::sleep_until(*instant)
				.then(|_| async move { message })
				.boxed()
		})
		.unwrap_or_else(|| future::pending().boxed())
}
