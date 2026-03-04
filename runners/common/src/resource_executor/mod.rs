use std::{marker::PhantomData, sync::Arc};

use models::rbac::ResourceType;
use tokio::{sync::Notify, task::JoinHandle};
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
	/// The update notifier that will be used to notify the task when the
	/// resource is updated.
	update_notifier: Arc<Notify>, // TODO: Is there any way to not use an Arc here?
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
		let update_notifier = Arc::new(Notify::new());
		let task = Self::start_task(
			resource_id,
			resource_type,
			state.clone(),
			cancellation_token.clone(),
			update_notifier.clone(),
		);
		Self {
			resource_id,
			resource_type,
			state,
			task,
			runner_executor: PhantomData,
			cancellation_token,
			update_notifier,
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
	pub(crate) async fn stop(self) {
		self.cancel();
		_ = self.task.await;
	}

	/// Returns the resource ID of the resource executor task.
	pub(crate) fn resource_id(&self) -> Uuid {
		self.resource_id
	}

	/// Ensures that the task is running. If it is not running, then start it.
	pub(crate) fn ensure_running(&mut self) -> &mut Self {
		// Ensure that the task is running. If it is not running, then start it.
		if self.task.is_finished() {
			self.task = Self::start_task(
				self.resource_id,
				self.resource_type,
				self.state.clone(),
				self.cancellation_token.clone(),
				self.update_notifier.clone(),
			);
		}
		self
	}

	/// Notifies the task that the resource has been updated.
	/// This will wake up the task and it will check for updates.
	#[tracing::instrument(skip(self), fields(resource_id = %self.resource_id))]
	pub(crate) fn notify_update(&self) {
		self.update_notifier.notify_waiters();
	}

	/// Starts the resource executor task. This will be used to start the task
	/// that will be used to manage the resource.
	fn start_task(
		resource_id: Uuid,
		resource_type: ResourceType,
		state: AppState<E>,
		cancellation_token: CancellationToken,
		update_notifier: Arc<Notify>,
	) -> JoinHandle<()> {
		tokio::spawn(async move {
			let resource_id = resource_id;
			let resource_type = resource_type;
			let state = state;
			let cancellation_token = cancellation_token;
			let update_notifier = update_notifier;

			match resource_type {
				ResourceType::Deployment => {
					deployment::handle_deployment(
						resource_id,
						state,
						cancellation_token,
						update_notifier,
					)
					.await;
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
				ResourceType::ManagedURL |
				ResourceType::Role => {
					todo!()
				}
			}

			debug!("Resource executor task completed for resource {resource_id}");
		})
	}
}
