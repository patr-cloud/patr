use std::{
	collections::BTreeMap,
	sync::{Arc, Mutex},
};

use common::prelude::*;
use futures::stream;
use models::api::workspace::deployment::*;

/// Record of a single executor call, used for assertions in tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorCall {
	Upsert(Uuid),
	Delete(Uuid),
	GetStatus(Uuid),
	ListRunning,
}

/// Shared mutable state backing all MockExecutor instances in a test.
///
/// Each field controls a different aspect of the mock:
/// - `calls`: append-only log of every method invocation, in order.
///    Tests assert on this to verify the actor called the right methods.
/// - `statuses`: what `get_deployment_status()` returns per deployment.
///    Default (missing key) returns `DeploymentStatus::Stopped`.
/// - `upsert_errors` / `delete_errors`: if set for a deployment ID,
///    the corresponding method returns this error string as `RunnerError`.
/// - `running`: the list of UUIDs returned by `list_running_deployments()`.
pub struct MockExecutorState {
	pub calls: Mutex<Vec<ExecutorCall>>,
	pub statuses: Mutex<BTreeMap<Uuid, DeploymentStatus>>,
	pub upsert_errors: Mutex<BTreeMap<Uuid, String>>,
	pub delete_errors: Mutex<BTreeMap<Uuid, String>>,
	pub running: Mutex<Vec<Uuid>>,
}

impl MockExecutorState {
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			calls: Mutex::new(Vec::new()),
			statuses: Mutex::new(BTreeMap::new()),
			upsert_errors: Mutex::new(BTreeMap::new()),
			delete_errors: Mutex::new(BTreeMap::new()),
			running: Mutex::new(Vec::new()),
		})
	}

	pub fn call_count(&self, predicate: impl Fn(&ExecutorCall) -> bool) -> usize {
		self.calls.lock().unwrap().iter().filter(|c| predicate(c)).count()
	}

	pub fn has_call(&self, predicate: impl Fn(&ExecutorCall) -> bool) -> bool {
		self.call_count(predicate) > 0
	}
}

pub struct MockExecutor {
	state: Arc<MockExecutorState>,
}

impl RunnerExecutor for MockExecutor {
	type InitializedState = Arc<MockExecutorState>;
	type Settings = ();

	fn runner_exposure_type(_: &RunnerSettings<Self::Settings>) -> RunnerExposureType {
		RunnerExposureType::Private
	}

	async fn initialize(
		_: &RunnerSettings<Self::Settings>,
	) -> Result<Self::InitializedState, RunnerError> {
		Ok(MockExecutorState::new())
	}

	async fn new(_: &RunnerSettings<Self::Settings>, state: Self::InitializedState) -> Self {
		Self { state }
	}

	async fn upsert_deployment(
		&self,
		deployment: WithId<Deployment>,
		_running_details: DeploymentRunningDetails,
	) -> Result<(), RunnerError> {
		let id = deployment.id;
		self.state.calls.lock().unwrap().push(ExecutorCall::Upsert(id));

		if let Some(err_msg) = self.state.upsert_errors.lock().unwrap().get(&id) {
			return Err(RunnerError::host(std::io::Error::other(err_msg.clone())));
		}
		Ok(())
	}

	async fn delete_deployment(&self, deployment_id: Uuid) -> Result<(), RunnerError> {
		self.state
			.calls
			.lock()
			.unwrap()
			.push(ExecutorCall::Delete(deployment_id));

		if let Some(err_msg) = self.state.delete_errors.lock().unwrap().get(&deployment_id) {
			return Err(RunnerError::host(std::io::Error::other(err_msg.clone())));
		}
		Ok(())
	}

	async fn list_running_deployments<'a>(&self) -> impl futures::Stream<Item = Uuid> + 'a {
		self.state.calls.lock().unwrap().push(ExecutorCall::ListRunning);
		let ids = self.state.running.lock().unwrap().clone();
		stream::iter(ids)
	}

	async fn get_deployment_status(
		&self,
		deployment_id: Uuid,
	) -> Result<DeploymentStatus, RunnerError> {
		self.state
			.calls
			.lock()
			.unwrap()
			.push(ExecutorCall::GetStatus(deployment_id));

		Ok(self
			.state
			.statuses
			.lock()
			.unwrap()
			.get(&deployment_id)
			.copied()
			.unwrap_or(DeploymentStatus::Stopped))
	}

	async fn next_deployment_status(
		&self,
		_deployment_id: Uuid,
	) -> Result<DeploymentStatus, RunnerError> {
		// Never resolves — tests use polling via CheckStatus instead.
		std::future::pending().await
	}
}
