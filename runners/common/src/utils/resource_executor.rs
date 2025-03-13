use std::marker::PhantomData;

use models::rbac::ResourceType;
use tokio::task::JoinHandle;

use crate::prelude::*;

/// The resource executor task that will be used to manage the resources. This
/// will be used to spawn the tasks that will be used to manage the resources.
pub struct ResourceExecutorTask<E>
where
	E: RunnerExecutor + Send + 'static,
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
	E: RunnerExecutor + Send + 'static,
{
	/// Creates a new resource executor task.
	#[tracing::instrument(skip(state))]
	pub(crate) fn new(resource_id: Uuid, resource_type: ResourceType, state: &AppState<E>) -> Self {
		let state = state.clone();
		Self {
			resource_id,
			runner_executor: PhantomData,
			task: tokio::spawn(async move {
				let resource_id = resource_id;
				let resource_type = resource_type;
				let state = state;

				let executor = E::new(&state.config, state.runner_state).await;

				match resource_type {
					ResourceType::Deployment => {
						// Keep checking for the status of the deployment and
						// update the database
						loop {
							let Ok(status) = executor.get_deployment_status(resource_id).await
							else {
								continue;
							};

							// Update the status of the deployment in the database
							let _ = query(
								r#"
								UPDATE
									deployment
								SET
									status = $1
								WHERE
									id = $2;
								"#,
							)
							.bind(status)
							.bind(resource_id)
							.execute(&state.database)
							.await;
						}
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

	/// Stops the resource executor task.
	#[tracing::instrument(skip(self), fields(resource_id = %self.resource_id))]
	pub(crate) async fn stop(&self) -> Result<(), RunnerError> {
		todo!()
	}

	pub(crate) fn resource_id(&self) -> Uuid {
		self.resource_id
	}
}
