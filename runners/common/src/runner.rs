use std::{collections::HashMap, future::IntoFuture, pin::pin, str::FromStr};

use futures::{
	future::{self, BoxFuture, Either},
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

use crate::{delayed_future::DelayedFuture, prelude::*};

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
	/// A list of resources that need to be reconciled at a later time
	/// This is used to retry resources that failed to reconcile
	reconciliation_list: Vec<DelayedFuture<Uuid>>,
	/// The future that will resolve to the next resource that needs to be
	/// reconciled
	next_reconcile_future: BoxFuture<'static, Uuid>,
	/// The bearer token for the runner to access the API
	authorization: BearerToken,
	/// The user agent that the runner uses to access the API
	user_agent: UserAgent,
}

impl<'de, E> Runner<'de, E>
where
	E: RunnerExecutor,
{
	/// Create a new runner with the given executor. This function will create a
	/// new database connection pool and set up the global default subscriber
	/// for the runner.
	pub fn new(executor: E) -> Result<Self, ErrorType> {
		let settings = RunnerSettings::<E::Settings<'_>>::parse(E::RUNNER_INTERNAL_NAME)
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
		let reconciliation_list = Vec::new();
		let next_reconcile_future = future::pending().boxed();
		let authorization = BearerToken::from_str(&settings.api_token)?;
		let user_agent = UserAgent::from_str(concat!(
			env!("CARGO_PKG_NAME"),
			"/",
			env!("CARGO_PKG_VERSION"),
		))?;

		Ok(Self {
			executor,
			settings,
			deployments,
			reconciliation_list,
			next_reconcile_future,
			authorization,
			user_agent,
		})
	}

	/// Run the runner. This function will start the runner and run the
	/// resources that the runner is responsible for. This function will run
	/// forever until the runner is stopped.
	pub async fn run(mut self) {
		info!("Runner started");
		let workspace_id = self.settings.workspace_id;
		let runner_id = self.settings.runner_id;

		let mut exit_signal = pin!(exit_signal());
		debug!("Exit signal listener started");

		info!("Connecting to the server");
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
							authorization: self.authorization.clone(),
							user_agent: self.user_agent.clone(),
						})
						.query(())
						.body(Default::default())
						.build(),
				)),
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
			self.reconciliation_list.clear();
			self.reconcile_all().await;

			// Reconcile all resources at a fixed interval
			let mut reconcile_all = Box::pin(time::sleep(E::FULL_RECONCILIATION_INTERVAL));
			let mut pinned_stream = pin!(stream);

			'message: loop {
				let Some(reconcile_all_or_one) = future::select(
					&mut exit_signal,
					future::select(
						reconcile_all.as_mut(),
						future::select(pinned_stream.next(), &mut self.next_reconcile_future),
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
						self.reconciliation_list.clear();
						self.reconcile_all().await;
						continue 'message;
					}
					Either::Right((actionable_message, _)) => actionable_message,
				};

				match reconcile_message {
					// Reconcile a resource from the server
					Either::Left((Some(Ok(response)), _)) => {
						// Reconcile the resource
						self.handle_server_message(response).await;
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
					Either::Right((deployment_id, _)) => {
						self.reconcile_deployment(deployment_id).await;
					}
				}
			}
		}

		info!("Runner stopped. Exiting...");
	}

	/// Reconcile all the resources that the runner is responsible for. This
	/// function will run the reconciliation for all the resources that the
	/// runner is responsible for.
	async fn reconcile_all(&mut self) {
		// Reconcile all resources
		self.reconcile_all_deployments().await;
	}

	/// Reconcile all the deployments that the runner is responsible for. This
	/// function will run the reconciliation for all the deployments that the
	/// runner is responsible for.
	async fn reconcile_all_deployments(&mut self) {
		// Reconcile all deployments
		info!("Reconciling all deployments");

		// Update running deployments
		let mut should_run_deployments = vec![];

		loop {
			trace!(
				"Fetching deployments {} to {}",
				should_run_deployments.len(),
				should_run_deployments.len() + Paginated::<()>::DEFAULT_PAGE_SIZE
			);
			let Ok(response) = client::make_request(
				ApiRequest::<ListDeploymentRequest>::builder()
					.path(ListDeploymentPath {
						workspace_id: self.settings.workspace_id,
					})
					.headers(ListDeploymentRequestHeaders {
						authorization: self.authorization.clone(),
						user_agent: self.user_agent.clone(),
					})
					.query(Paginated {
						data: (),
						count: Paginated::<()>::DEFAULT_PAGE_SIZE,
						page: should_run_deployments.len() / Paginated::<()>::DEFAULT_PAGE_SIZE,
					})
					.body(ListDeploymentRequest)
					.build(),
			)
			.await
			.inspect_err(|err| {
				debug!("Failed to get deployment list: {:?}", err);
				warn!("Using the pre-existing list");
			}) else {
				should_run_deployments = self.deployments.keys().copied().collect();
				break;
			};

			should_run_deployments.extend(
				response
					.body
					.deployments
					.into_iter()
					.map(|deployment| deployment.id)
					.collect::<Vec<_>>(),
			);

			if response.headers.total_count.0 <= should_run_deployments.len() {
				break;
			}
		}

		let mut running_deployments = pin!(self.executor.list_running_deployments().await);

		while let Some(deployment_id) = running_deployments.next().await {
			let deployment = should_run_deployments
				.iter()
				.find(|&&id| deployment_id == id);

			// If the deployment does not exist in the should run list, delete it
			let Some(&deployment_id) = deployment else {
				trace!(
					"Deployment `{}` does not exist in the should run list",
					deployment_id
				);
				info!("Deleting deployment `{}`", deployment_id);

				if let Err(wait_time) = self.executor.delete_deployment(deployment_id).await {
					self.reconciliation_list.push(DelayedFuture::new(
						Instant::now() + wait_time,
						deployment_id,
					));
					self.recheck_next_reconcile_future();
				}
				return;
			};

			// If it does exist, reconcile the deployment and remove it from the should run
			// list
			self.reconcile_deployment(deployment_id).await;
			should_run_deployments.retain(|&id| id != deployment_id);
		}

		// All remaining deployments are the ones that are there in the should run list,
		// but aren't running. So get them up and running
		for deployment_id in should_run_deployments {
			self.reconcile_deployment(deployment_id).await;
		}
	}

	/// Reconcile a specific deployment. This function will run the
	/// reconciliation for a specific deployment (based on the ID)
	async fn reconcile_deployment(&mut self, deployment_id: Uuid) {
		trace!("Reconciling deployment `{}`", deployment_id);
		self.reconciliation_list
			.retain(|message| message.value() != &deployment_id);

		let result = 'reconcile: {
			let Ok(GetDeploymentInfoResponse {
				deployment,
				running_details,
			}) = client::make_request(
				ApiRequest::<GetDeploymentInfoRequest>::builder()
					.path(GetDeploymentInfoPath {
						workspace_id: self.settings.workspace_id,
						deployment_id,
					})
					.headers(GetDeploymentInfoRequestHeaders {
						authorization: self.authorization.clone(),
						user_agent: self.user_agent.clone(),
					})
					.query(())
					.body(GetDeploymentInfoRequest)
					.build(),
			)
			.await
			.map(|response| response.body)
			.inspect_err(|err| {
				debug!(
					"Failed to get deployment info for `{}`: {:?}",
					deployment_id, err
				);
				debug!("Retrying in 5 seconds");
			})
			else {
				break 'reconcile Err(Duration::from_secs(5));
			};

			if let Err(err) = self
				.executor
				.upsert_deployment(deployment, running_details)
				.await
			{
				break 'reconcile Err(err);
			};

			Ok(())
		};

		if let Err(wait_time) = result {
			self.reconciliation_list.push(DelayedFuture::new(
				Instant::now() + wait_time,
				deployment_id,
			));
		}

		self.recheck_next_reconcile_future();
	}

	/// Handle a message from the server. This function will handle the message
	/// from the server and run the reconciliation for the resource that the
	/// message is for.
	async fn handle_server_message(&mut self, msg: StreamRunnerDataForWorkspaceServerMsg) {
		info!("Handling server message: {:?}", msg);
		// if this resource is already queued for reconciliation, remove that
		let current_id = get_resource_id_from_message(&msg);
		self.reconciliation_list
			.retain(|message| message.value() != &current_id);

		use StreamRunnerDataForWorkspaceServerMsg::*;
		let response = match msg.clone() {
			DeploymentCreated {
				deployment,
				running_details,
			} => {
				self.executor
					.upsert_deployment(deployment, running_details)
					.await
			}
			DeploymentUpdated {
				deployment,
				running_details,
			} => {
				self.executor
					.upsert_deployment(deployment, running_details)
					.await
			}
			DeploymentDeleted { id } => self.executor.delete_deployment(id).await,
		};
		if let Err(wait_time) = response {
			self.reconciliation_list.push(DelayedFuture::new(
				Instant::now() + wait_time,
				get_resource_id_from_message(&msg),
			));
		}

		self.recheck_next_reconcile_future();
	}

	/// Get the next reconcile future from the list of futures. This function
	/// will get the future that resolves to the earliest reconciliation future
	/// and set that to `next_reconcile_future`.
	fn recheck_next_reconcile_future(&mut self) {
		self.next_reconcile_future = self
			.reconciliation_list
			.iter()
			.reduce(|a, b| {
				if a.resolve_at() < b.resolve_at() {
					a
				} else {
					b
				}
			})
			.map(|message| message.clone().into_future().boxed())
			.unwrap_or_else(|| future::pending().boxed());
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
