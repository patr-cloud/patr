use std::future::Future;

use futures::Stream;
use models::api::workspace::deployment::*;
use serde::{Serialize, de::DeserializeOwned};

use crate::prelude::*;

/// This trait is the main trait that the runner needs to implement to run the
/// resources.
pub trait RunnerExecutor: Sized {
	/// The settings type for the runner. This is used to store any additional
	/// settings needed for the runner.
	type Settings: Serialize + DeserializeOwned + Clone + Send + Sync + 'static;

	/// The type that the runner will initialize in the
	/// [`initialize`][RunnerExecutor::initialize] function, and will be passed
	/// to the [`new`][RunnerExecutor::new] function upon each instantiation.
	type InitializedState: Clone + Send + Sync + 'static;

	/// The internal name of the runner. This is used to identify the runner in
	/// tracing and logs.
	fn runner_internal_name() -> String {
		std::env::current_exe()
			.ok()
			.and_then(|pb| pb.file_name().map(|f| f.to_string_lossy().to_string()))
			.unwrap_or("unknown".to_string())
	}

	/// This function is called when the runner is initialized. This is where
	/// the runner should initialize any resources it needs to run the
	/// resources. This function is guaranteed to be called only once.
	fn initialize(
		_: &RunnerSettings<Self::Settings>,
	) -> impl Future<Output = Result<Self::InitializedState, RunnerError>> + Send;

	/// This function is called when the runner is constructed. This function
	/// will be called multiple times when data needs to be extracted from the
	/// runner. So this function should be lightweight and quick to run. Any
	/// heavy initialization should be done in the
	/// [`initialize`][RunnerExecutor::initialize] function.
	fn new(
		settings: &RunnerSettings<Self::Settings>,
		state: Self::InitializedState,
	) -> impl Future<Output = Self> + Send;

	/// This function is called when a deployment is created, or updated.
	/// The runner should return an error if the deployment failed to start.
	/// This will be used to retry the deployment.
	fn upsert_deployment(
		&self,
		deployment: WithId<Deployment>,
		running_details: DeploymentRunningDetails,
	) -> impl Future<Output = Result<(), RunnerError>> + Send;

	/// This function is called when a deployment is deleted. The runner should
	/// return an error if the deployment failed to delete. This will be used to
	/// retry the deletion.
	fn delete_deployment(
		&self,
		deployment_id: Uuid,
	) -> impl Future<Output = Result<(), RunnerError>> + Send;

	/// This function should return a stream of all the running deployment IDs
	/// in the runner, sorted by the deployment ID.
	fn list_running_deployments<'a>(
		&self,
	) -> impl Future<Output = impl Stream<Item = Uuid> + 'a> + Send;

	/// This function should return the status of the deployment. This function
	/// will be called when the runner is reconciling the deployments to get the
	/// status of the deployment.
	fn get_deployment_status(
		&self,
		deployment_id: Uuid,
	) -> impl Future<Output = Result<DeploymentStatus, RunnerError>> + Send;
}
