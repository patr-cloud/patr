use std::{
	collections::{BTreeMap, BTreeSet},
	sync::{Arc, Mutex},
};

use common::prelude::*;
use futures::Stream;
use models::api::workspace::{deployment::*, runner::RunnerExposureType};

/// Record of a single executor call, used for assertions in tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorCall {
	Apply(String),
	Delete(String),
}

/// An object the mock backend holds.
#[derive(Debug, Clone)]
pub struct MockObject {
	pub owner: Owner,
	pub hash: String,
	pub spec: String,
	pub depends_on: BTreeSet<String>,
}

/// Shared mutable state backing all MockExecutor clones in a test.
///
/// - `calls`: append-only log of every apply and delete, in order.
/// - `objects`: what exists on the mock backend, by ID.
/// - `apply_errors`: IDs whose apply fails.
/// - `in_use`: IDs reported as still in use, so they aren't deleted.
/// - `statuses`: what `observe()` reports per deployment.
pub struct MockExecutorState {
	pub calls: Mutex<Vec<ExecutorCall>>,
	pub objects: Mutex<BTreeMap<String, MockObject>>,
	pub apply_errors: Mutex<BTreeSet<String>>,
	pub in_use: Mutex<BTreeSet<String>>,
	pub statuses: Mutex<BTreeMap<Uuid, ObservedDeploymentStatus>>,
}

impl MockExecutorState {
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			calls: Mutex::new(Vec::new()),
			objects: Mutex::new(BTreeMap::new()),
			apply_errors: Mutex::new(BTreeSet::new()),
			in_use: Mutex::new(BTreeSet::new()),
			statuses: Mutex::new(BTreeMap::new()),
		})
	}

	pub fn call_count(&self, predicate: impl Fn(&ExecutorCall) -> bool) -> usize {
		self.calls
			.lock()
			.unwrap()
			.iter()
			.filter(|c| predicate(c))
			.count()
	}

	pub fn has_call(&self, predicate: impl Fn(&ExecutorCall) -> bool) -> bool {
		self.call_count(predicate) > 0
	}

	pub fn exists(&self, id: &str) -> bool {
		self.objects.lock().unwrap().contains_key(id)
	}

	pub fn spec(&self, id: &str) -> Option<String> {
		self.objects
			.lock()
			.unwrap()
			.get(id)
			.map(|object| object.spec.clone())
	}
}

/// A deployment's service.
pub fn service(deployment_id: Uuid) -> String {
	format!("service/{deployment_id}")
}

/// A deployment's default-URL routes.
pub fn routes(deployment_id: Uuid) -> String {
	format!("routes/{deployment_id}")
}

/// A deployment's volume at `path`.
pub fn volume(deployment_id: Uuid, path: &str) -> String {
	format!("volume/{deployment_id}{path}")
}

/// A deployment's config mount at `path`.
pub fn config(deployment_id: Uuid, path: &str) -> String {
	format!("config/{deployment_id}{path}")
}

/// A managed URL's route.
pub fn url(managed_url_id: Uuid) -> String {
	format!("url/{managed_url_id}")
}

/// The ingress the runner itself owns.
pub const INGRESS: &str = "service/ingress";

/// An executor over an in-memory backend. A deployment expects a volume per
/// path and its routes always, and while it should run, a config per mount and
/// a service depending on them. A managed URL expects its route while its
/// deployment is local. The runner expects an ingress importing every route.
#[derive(Clone)]
pub struct MockExecutor {
	pub state: Arc<MockExecutorState>,
}

impl RunnerExecutor for MockExecutor {
	type Id = String;
	type Inputs = ();
	type Settings = ();
	type Spec = String;

	fn runner_exposure_type(_: &RunnerSettings<Self::Settings>) -> RunnerExposureType {
		RunnerExposureType::Private
	}

	async fn initialize(_: &RunnerSettings<Self::Settings>) -> Result<Self, RunnerError> {
		Ok(Self {
			state: MockExecutorState::new(),
		})
	}

	async fn update_ingress_token(&self, _: String) -> Result<bool, RunnerError> {
		Ok(false)
	}

	async fn inputs(&self, _: Owner) -> Result<Self::Inputs, RunnerError> {
		Ok(())
	}

	fn expected(
		&self,
		owner: Owner,
		local: &LocalState,
		_: &Self::Inputs,
	) -> Result<ExpectedGraph<Self::Id, Self::Spec>, RunnerError> {
		let mut graph = ExpectedGraph::new(owner);
		match owner {
			Owner::Deployment(deployment_id) => {
				let Some((deployment, running_details)) = local.deployments.get(&deployment_id)
				else {
					return Ok(graph);
				};
				let volumes = running_details
					.volumes
					.keys()
					.map(|path| graph.add(volume(deployment_id, path), String::from("volume"), []))
					.collect::<Vec<_>>();
				graph.add(
					routes(deployment_id),
					format!("routes should_run={}", deployment.should_run),
					[],
				);
				if deployment.should_run {
					let configs = running_details
						.config_mounts
						.iter()
						.map(|(path, content)| {
							graph.add(
								config(deployment_id, path),
								format!("config {:?}", content.as_slice()),
								[],
							)
						})
						.collect::<Vec<_>>();
					let env = running_details
						.environment_variables
						.iter()
						.map(|(name, value)| match value {
							EnvironmentVariableValue::String(value) => format!("{name}={value}"),
							EnvironmentVariableValue::Secret { from_secret } => format!(
								"{name}=secret:{from_secret}@{:?}",
								local.secrets.get(from_secret)
							),
						})
						.collect::<Vec<_>>()
						.join(",");
					graph.add(
						service(deployment_id),
						format!("service image={} env={env}", deployment.image_tag),
						configs.into_iter().chain(volumes),
					);
				}
			}
			Owner::ManagedUrl(managed_url_id) => {
				if let Some(managed_url) = local.managed_urls.get(&managed_url_id) &&
					let Some((deployment, _)) = local.deployments.get(&managed_url.deployment_id)
				{
					graph.add(
						url(managed_url_id),
						format!(
							"url {}{} should_run={}",
							managed_url.host, managed_url.path, deployment.should_run
						),
						[],
					);
				}
			}
			Owner::System => {
				let imports = local
					.deployments
					.keys()
					.map(|id| routes(*id))
					.chain(local.managed_urls.keys().map(|id| url(*id)))
					.collect::<Vec<_>>()
					.join(",");
				graph.add(
					String::from(INGRESS),
					format!("ingress imports={imports}"),
					[],
				);
			}
		}
		Ok(graph)
	}

	async fn list(&self, owner: Option<Owner>) -> Result<Vec<Object<Self::Id>>, RunnerError> {
		Ok(self
			.state
			.objects
			.lock()
			.unwrap()
			.iter()
			.filter(|(_, object)| owner.is_none_or(|owner| owner == object.owner))
			.map(|(id, object)| Object {
				id: id.clone(),
				owner: object.owner,
				hash: object.hash.clone(),
				depends_on: object.depends_on.clone(),
			})
			.collect())
	}

	async fn apply(
		&self,
		owner: Owner,
		id: &Self::Id,
		spec: &Self::Spec,
		hash: &str,
	) -> Result<(), RunnerError> {
		self.state
			.calls
			.lock()
			.unwrap()
			.push(ExecutorCall::Apply(id.clone()));
		if self.state.apply_errors.lock().unwrap().contains(id) {
			return Err(RunnerError::host(format!("failed to apply {id}")));
		}

		let mut objects = self.state.objects.lock().unwrap();
		// A service mounts the configs and volumes of its deployment that
		// exist.
		let depends_on = match id.strip_prefix("service/") {
			Some(deployment_id) => objects
				.keys()
				.filter(|other| {
					other.starts_with(&format!("config/{deployment_id}")) ||
						other.starts_with(&format!("volume/{deployment_id}"))
				})
				.cloned()
				.collect(),
			None => BTreeSet::new(),
		};
		objects.insert(
			id.clone(),
			MockObject {
				owner,
				hash: String::from(hash),
				spec: spec.clone(),
				depends_on,
			},
		);
		Ok(())
	}

	async fn in_use(&self, id: &Self::Id) -> Result<bool, RunnerError> {
		Ok(self.state.in_use.lock().unwrap().contains(id))
	}

	async fn delete(&self, id: &Self::Id) -> Result<DeleteOutcome, RunnerError> {
		self.state
			.calls
			.lock()
			.unwrap()
			.push(ExecutorCall::Delete(id.clone()));
		self.state.objects.lock().unwrap().remove(id);
		Ok(DeleteOutcome::Deleted)
	}

	async fn observe(&self) -> Result<BTreeMap<Uuid, ObservedDeploymentStatus>, RunnerError> {
		Ok(self.state.statuses.lock().unwrap().clone())
	}

	fn watch(&self) -> impl Stream<Item = Owner> + Send + 'static {
		futures::stream::pending()
	}
}
