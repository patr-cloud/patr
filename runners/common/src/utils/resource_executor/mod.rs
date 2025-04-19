use std::marker::PhantomData;

use models::rbac::ResourceType;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::prelude::*;

/// The resource executor task that will be used to specifically manage the
/// deployments of the runner.
mod deployment;

/// The resource executor task that will be used to manage the resources. This
/// will be used to spawn the tasks that will be used to manage the resources.
pub struct ResourceExecutorTask<E>
where
	E: RunnerExecutor + Send + 'static,
{
	/// The ID of the resource that will be managed.
	resource_id: Uuid,
	/// The type of the resource that will be managed.
	resource_type: ResourceType,
	/// The state of the application that will be used to manage the resources.
	state: AppState<E>,
	/// The task that will be used to manage the resources.
	task: JoinHandle<()>,
	/// The type of runner executor that will be used to manage the resources.
	runner_executor: PhantomData<E>,
	/// The cancellation token that will be used to cancel the task.
	cancellation_token: CancellationToken,
}

impl<E> ResourceExecutorTask<E>
where
	E: RunnerExecutor + Send + 'static,
{
	/// Creates a new resource executor task.
	#[tracing::instrument(skip(state))]
	pub(crate) fn new(resource_id: Uuid, resource_type: ResourceType, state: AppState<E>) -> Self {
		let cancellation_token = crate::runner::GLOBAL_CANCEL_TOKEN
			.get_or_init(CancellationToken::new)
			.child_token();
		let task = Self::start_task(
			resource_id,
			resource_type,
			state.clone(),
			cancellation_token.clone(),
		);
		Self {
			resource_id,
			runner_executor: PhantomData,
			resource_type,
			state,
			task,
			cancellation_token,
		}
	}

	/// Creates a new resource executor task for a deployment.
	pub(crate) fn new_deployment(deployment_id: Uuid, state: AppState<E>) -> Self {
		Self::new(deployment_id, ResourceType::Deployment, state)
	}

	/// Cancels the resource executor task.
	pub(crate) fn cancel(&self) {
		self.cancellation_token.cancel();
	}

	/// Stops the resource executor task.
	#[tracing::instrument(skip(self), fields(resource_id = %self.resource_id))]
	pub(crate) async fn stop(self) -> Result<(), RunnerError> {
		self.cancel();
		_ = self.task.await;
		Ok(())
	}

	/// Returns the resource ID of the resource executor task.
	pub(crate) fn resource_id(&self) -> Uuid {
		self.resource_id
	}

	/// Ensures that the task is running. If it is not running, then start it.
	pub(crate) fn ensure_running(&mut self) -> Result<(), RunnerError> {
		// Ensure that the task is running. If it is not running, then start it.
		if self.task.is_finished() {
			self.task = Self::start_task(
				self.resource_id,
				self.resource_type,
				self.state.clone(),
				self.cancellation_token.clone(),
			);
		}
		Ok(())
	}

	/// Starts the resource executor task. This will be used to start the task
	/// that will be used to manage the resource.
	fn start_task(
		resource_id: Uuid,
		resource_type: ResourceType,
		state: AppState<E>,
		cancellation_token: CancellationToken,
	) -> JoinHandle<()> {
		tokio::spawn(async move {
			let resource_id = resource_id;
			let resource_type = resource_type;
			let state = state;
			let cancellation_token = cancellation_token;

			let executor = E::new(&state.config, state.runner_state.clone()).await;

			match resource_type {
				ResourceType::Deployment => {
					_ = deployment::handle_deployment(
						resource_id,
						executor,
						state,
						cancellation_token,
					)
					.await
					.inspect_err(|err| {
						tracing::error!("Failed to handle deployment resource: {}", err);
					});
				}
				ResourceType::Workspace |
				ResourceType::Project |
				ResourceType::Runner |
				ResourceType::Volume |
				ResourceType::Database |
				ResourceType::StaticSite |
				ResourceType::ContainerRegistryRepository |
				ResourceType::Secret |
				ResourceType::Domain |
				ResourceType::DnsRecord |
				ResourceType::ManagedURL => {
					todo!()
				}
			}
		})
	}
}
