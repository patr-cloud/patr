use models::rbac::ResourceType;
use tokio::task::JoinHandle;

use crate::prelude::*;

/// The resource executor task that will be used to manage the resources. This
/// will be used to spawn the tasks that will be used to manage the resources.
pub struct ResourceExecutorTask {
	/// The task that will be used to manage the resources.
	task: JoinHandle<()>,
	/// The ID of the resource that is being managed.
	resource_id: Uuid,
	/// The type of the resource that is being managed.
	resource_type: ResourceType,
}

impl ResourceExecutorTask {
	/// Creates a new resource executor task.
	pub(crate) fn new(resource_id: Uuid, resource_type: ResourceType) -> Self {
		Self {
			resource_id,
			resource_type,
			task: tokio::spawn(async move {
				let resource_id = resource_id;
				let resource_type = resource_type;
			}),
		}
	}

	pub(crate) async fn stop(&self) {
		todo!()
	}
}
