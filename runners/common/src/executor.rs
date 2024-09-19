use std::{future::Future, time::Duration};

use futures::Stream;
use models::api::workspace::deployment::*;
use serde::{de::DeserializeOwned, Serialize};

use crate::prelude::*;

/// This trait is the main trait that the runner needs to implement to run the
/// resources.
pub trait RunnerExecutor: Sized {
	/// The reconciliation interval for the runner. This is the interval at
	/// which the runner will reconcile ALL the resources with the server. The
	/// default is 10 minutes.
	const FULL_RECONCILIATION_INTERVAL: Duration = Duration::from_secs(10 * 60);

	/// The internal name of the runner. This is used to identify the runner in
	/// tracing and logs.
	const RUNNER_INTERNAL_NAME: &'static str;

	/// The settings type for the runner. This is used to store any additional
	/// settings needed for the runner.
	type Settings: Serialize + DeserializeOwned + Clone + Send + Sync;

	/// This function is called when the runner is constructed. This is where
	/// the runner should initialize any resources it needs to run the
	/// deployments.
	fn create(settings: &RunnerSettings<Self::Settings>) -> impl Future<Output = Self>;

	/// This function is called when a deployment is created, or updated.
	/// The runner should return an error with a duration if the deployment
	/// failed to reconcile. This will be used to retry the deployment after
	/// the given duration.
	fn upsert_deployment(
		&self,
		deployment: WithId<Deployment>,
		running_details: DeploymentRunningDetails,
	) -> impl Future<Output = Result<(), Duration>>;

	/// This function is called when a deployment is deleted. The runner should
	/// return an error with a duration if the deployment failed to delete. This
	/// will be used to retry the deployment after the given duration.
	fn delete_deployment(&self, deployment_id: Uuid) -> impl Future<Output = Result<(), Duration>>;

	/// This function should return a stream of all the running deployment IDs
	/// in the runner, sorted by the deployment ID.
	fn list_running_deployments<'a>(&self) -> impl Future<Output = impl Stream<Item = Uuid> + 'a>;
}
