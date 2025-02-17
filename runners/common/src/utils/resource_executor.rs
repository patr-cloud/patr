use std::{
	hash::{Hash, Hasher},
	marker::PhantomData,
};

use models::rbac::ResourceType;
use tokio::task::JoinHandle;

use crate::prelude::*;

/// The resource executor task that will be used to manage the resources. This
/// will be used to spawn the tasks that will be used to manage the resources.
pub struct ResourceExecutorTask<E>
where
	E: RunnerExecutor,
{
	/// The ID of the resource that will be managed.
	resource_id: Uuid,
	/// The task that will be used to manage the resources.
	task: JoinHandle<()>,
	/// The type of runner executor that will be used to manage the resources.
	runner_executor: PhantomData<E>,
}

impl<E> ResourceExecutorTask<E>
where
	E: RunnerExecutor,
{
	/// Creates a new resource executor task.
	pub(crate) fn new(resource_id: Uuid, resource_type: ResourceType, state: &AppState<E>) -> Self {
		let runner_state = state.runner_state.clone();
		let config = state.config.clone();
		Self {
			resource_id,
			runner_executor: PhantomData,
			task: tokio::spawn(async move {
				let resource_id = resource_id;
				let resource_type = resource_type;
				let state = runner_state;
				let config = config;

				let executor = E::new(&config, state).await;

				match resource_type {
					ResourceType::Deployment => {
						// Keep checking for the status of the deployment and
						// update the database
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
			}),
		}
	}

	pub(crate) async fn stop(&self) {
		todo!()
	}

	pub(crate) fn is_running(&self) -> bool {
		!self.task.is_finished()
	}

	pub(crate) fn resource_id(&self) -> Uuid {
		self.resource_id
	}
}
