use std::future::Future;

use futures::Stream;
use models::{api::workspace::deployment::*, prelude::*};
use serde::{Deserialize, Serialize};

/// This trait is the main trait that the runner needs to implement to run the
/// resources.
pub trait RunnerExecutor {
	/// The settings type for the runner. This is used to store any additional
	/// settings needed for the runner.
	type Settings<'de>: Serialize + Deserialize<'de>;

	/// This function is called when a deployment is created. The runner should
	/// start the deployment.
	fn create_deployment(
		&self,
		deployment: WithId<Deployment>,
		running_details: DeploymentRunningDetails,
	) -> impl Future<Output = Result<(), ErrorType>>;
	/// This function should return a stream of all the running deployment IDs
	/// in the runner.
	fn list_running_deployments(&self) -> impl Stream<Item = Uuid>;
	/// This function is called when a deployment is updated. The runner should
	/// update the deployment.
	fn update_deployment(
		&self,
		deployment: WithId<Deployment>,
		running_details: DeploymentRunningDetails,
	) -> impl Future<Output = Result<(), ErrorType>>;
	/// This function is called when a deployment is deleted. The runner should
	/// delete the deployment.
	fn delete_deployment(&self, id: Uuid) -> impl Future<Output = Result<(), ErrorType>>;
}
