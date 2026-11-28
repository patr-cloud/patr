use std::{
	collections::{BTreeMap, BTreeSet},
	fmt::{self, Debug, Display},
	future::Future,
	str::FromStr,
};

use futures::Stream;
use models::api::workspace::{deployment::*, runner::RunnerExposureType};
use serde::{Serialize, de::DeserializeOwned};
use time::OffsetDateTime;

use crate::prelude::*;

/// Who an object belongs to. The controller's work queue holds owners: an
/// owner in the queue means its objects may be out of date. Stored on each
/// object as its `patr.owner` label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Owner {
	/// A deployment, which owns its service, config mounts, volumes and ingress
	/// snippet.
	Deployment(Uuid),
	/// A managed URL, which owns its ingress snippet.
	ManagedUrl(Uuid),
	/// The runner itself, which owns the ingress, the tunnel, Alloy and the
	/// network.
	System,
}

impl Display for Owner {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Deployment(id) => write!(f, "deployment:{id}"),
			Self::ManagedUrl(id) => write!(f, "managedUrl:{id}"),
			Self::System => write!(f, "system"),
		}
	}
}

impl FromStr for Owner {
	type Err = RunnerError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.split_once(':') {
			None if s == "system" => Ok(Self::System),
			Some(("deployment", id)) => id
				.parse()
				.map(Self::Deployment)
				.map_err(|_| RunnerError::host(format!("invalid owner `{s}`"))),
			Some(("managedUrl", id)) => id
				.parse()
				.map(Self::ManagedUrl)
				.map_err(|_| RunnerError::host(format!("invalid owner `{s}`"))),
			_ => Err(RunnerError::host(format!("invalid owner `{s}`"))),
		}
	}
}

/// An object that exists on the backend, as the executor lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object<Id> {
	/// The object's ID.
	pub id: Id,
	/// The object's owner, from its `patr.owner` label.
	pub owner: Owner,
	/// The hash of the spec the object was applied with, from its
	/// `patr.specHash` label.
	pub hash: String,
	/// The objects this one references, such as the configs and volumes a
	/// service mounts.
	pub depends_on: BTreeSet<Id>,
}

/// What should exist for one owner: the spec of each of its objects, and the
/// objects each one depends on. Dependencies never cross owners.
#[derive(Debug, Clone)]
pub struct ExpectedGraph<Id, Spec> {
	/// The owner of every object in the graph.
	pub owner: Owner,
	/// Each object's spec and the IDs of the objects it depends on.
	pub nodes: BTreeMap<Id, (Spec, BTreeSet<Id>)>,
}

impl<Id, Spec> ExpectedGraph<Id, Spec>
where
	Id: Ord + Clone + Display,
{
	/// An empty graph for `owner`.
	pub fn new(owner: Owner) -> Self {
		Self {
			owner,
			nodes: BTreeMap::new(),
		}
	}

	/// Adds an object and returns its ID, so the objects that depend on it can
	/// name it.
	pub fn add(&mut self, id: Id, spec: Spec, depends_on: impl IntoIterator<Item = Id>) -> Id {
		self.nodes
			.insert(id.clone(), (spec, depends_on.into_iter().collect()));
		id
	}

	/// The object IDs in an order that applies every object after the objects
	/// it depends on. Ties go in ID order, so the order is deterministic. Errors
	/// if an object depends on one outside the graph or the dependencies form a
	/// cycle, since either means the graph was built wrong.
	pub fn apply_order(&self) -> Result<Vec<Id>, RunnerError> {
		let mut unapplied = BTreeMap::<&Id, usize>::new();
		let mut dependents = BTreeMap::<&Id, Vec<&Id>>::new();
		for (id, (_, depends_on)) in &self.nodes {
			for dependency in depends_on {
				if !self.nodes.contains_key(dependency) {
					return Err(RunnerError::host(format!(
						"`{id}` depends on `{dependency}`, which isn't expected for `{}`",
						self.owner
					)));
				}
				dependents.entry(dependency).or_default().push(id);
			}
			unapplied.insert(id, depends_on.len());
		}

		let mut ready = unapplied
			.iter()
			.filter(|(_, count)| **count == 0)
			.map(|(id, _)| *id)
			.collect::<BTreeSet<_>>();
		let mut order = Vec::with_capacity(self.nodes.len());
		while let Some(id) = ready.pop_first() {
			order.push(id.clone());
			for dependent in dependents.get(id).into_iter().flatten() {
				if let Some(count) = unapplied.get_mut(dependent) {
					*count -= 1;
					if *count == 0 {
						ready.insert(dependent);
					}
				}
			}
		}

		if order.len() != self.nodes.len() {
			return Err(RunnerError::host(format!(
				"the objects expected for `{}` depend on each other in a cycle",
				self.owner
			)));
		}
		Ok(order)
	}
}

/// What deleting an object did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteOutcome {
	/// The object is gone, including if it was already gone.
	Deleted,
	/// The object is still in use, so it wasn't deleted. A later reconcile
	/// tries again.
	InUse,
}

/// A finalizer: a step that has to finish before an object can be deleted.
/// Each kind is a separate migration on `object_finalizer`, with any columns of
/// its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FinalizerKind {
	/// Confirms with the API that the deployment a volume belonged to is
	/// deleted or on another runner, so deleting the volume never destroys the
	/// data of a deployment this runner still runs.
	Volume,
}

impl Display for FinalizerKind {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Volume => write!(f, "volume"),
		}
	}
}

/// A managed URL as the runner stores it locally, with its host resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalManagedUrl {
	/// The fully-qualified host the URL is served on.
	pub host: String,
	/// The path the URL is mounted at.
	pub path: String,
	/// The deployment the URL proxies to.
	pub deployment_id: Uuid,
	/// The deployment's port the URL proxies to.
	pub port: u16,
}

/// The runner's local copy of what the API wants it to run, loaded fresh from
/// SQLite for each reconcile.
#[derive(Clone, Default)]
pub struct LocalState {
	/// Every deployment on this runner, with its running details.
	pub deployments: BTreeMap<Uuid, (Deployment, DeploymentRunningDetails)>,
	/// Every managed URL that proxies to a deployment on this runner.
	pub managed_urls: BTreeMap<Uuid, LocalManagedUrl>,
	/// When each secret's value last changed.
	pub secrets: BTreeMap<Uuid, OffsetDateTime>,
}

/// The trait a runner implements to run resources on a backend. The framework
/// works out what should exist for each [`Owner`] with
/// [`expected`](Self::expected), compares it with what
/// [`list`](Self::list) reports, applies what's missing or changed, and
/// deletes what's no longer expected.
pub trait RunnerExecutor: Clone + Send + Sync + 'static {
	/// The settings type for the runner. This is used to store any additional
	/// settings needed for the runner.
	type Settings: Serialize + DeserializeOwned + Clone + Send + Sync + 'static;

	/// The ID of an object on the backend.
	type Id: Ord + Clone + Debug + Display + FromStr + Send + Sync + 'static;

	/// What an object should look like. Hashed to decide whether an object
	/// needs applying, so it holds only what the object is built from, with
	/// late-bound values (image digests, secret values) left for
	/// [`apply`](Self::apply) to resolve.
	type Spec: Serialize + Clone + Send + Sync + 'static;

	/// Values an owner's expected objects depend on that only the backend
	/// holds, read by [`inputs`](Self::inputs) before
	/// [`expected`](Self::expected) is called.
	type Inputs: Send + Sync + 'static;

	/// The internal name of the runner. This is used to identify the runner in
	/// tracing and logs.
	#[must_use]
	fn runner_internal_name() -> String {
		std::env::current_exe()
			.ok()
			.and_then(|pb| pb.file_name().map(|f| f.to_string_lossy().to_string()))
			.unwrap_or_else(|| "unknown".to_string())
	}

	/// The exposure type of the runner. This is used to determine how the
	/// runner will expose the resources to the outside world.
	#[must_use]
	fn runner_exposure_type(_: &RunnerSettings<Self::Settings>) -> RunnerExposureType;

	/// Connects to the backend. Called once, at startup. Creates nothing beyond
	/// what the backend needs to run at all; everything else is reconciled.
	fn initialize(
		settings: &RunnerSettings<Self::Settings>,
	) -> impl Future<Output = Result<Self, RunnerError>> + Send;

	/// Stores the ingress token the runner's tunnel connects with. Returns
	/// whether it changed, in which case the runner's own objects need
	/// reconciling.
	fn update_ingress_token(
		&self,
		token: String,
	) -> impl Future<Output = Result<bool, RunnerError>> + Send;

	/// Reads the backend-held values `owner`'s expected objects depend on.
	fn inputs(
		&self,
		owner: Owner,
	) -> impl Future<Output = Result<Self::Inputs, RunnerError>> + Send;

	/// The objects that should exist for `owner`, given the local state.
	/// Pure: everything it needs is in `local` and `inputs`.
	fn expected(
		&self,
		owner: Owner,
		local: &LocalState,
		inputs: &Self::Inputs,
	) -> Result<ExpectedGraph<Self::Id, Self::Spec>, RunnerError>;

	/// The objects that exist for `owner`, or for every owner if `None`.
	/// Objects without a valid owner label are left out, so they're never
	/// reaped.
	fn list(
		&self,
		owner: Option<Owner>,
	) -> impl Future<Output = Result<Vec<Object<Self::Id>>, RunnerError>> + Send;

	/// Creates or updates an object to match `spec`, labelling it with `owner`
	/// and `hash`.
	fn apply(
		&self,
		owner: Owner,
		id: &Self::Id,
		spec: &Self::Spec,
		hash: &str,
	) -> impl Future<Output = Result<(), RunnerError>> + Send;

	/// The finalizers that must finish before the object can be deleted.
	fn finalizers(&self, id: &Self::Id) -> Vec<FinalizerKind>;

	/// Whether something still uses the object, in which case it's left for a
	/// later reconcile.
	fn in_use(&self, id: &Self::Id) -> impl Future<Output = Result<bool, RunnerError>> + Send;

	/// Deletes an object.
	fn delete(
		&self,
		id: &Self::Id,
	) -> impl Future<Output = Result<DeleteOutcome, RunnerError>> + Send;

	/// The observed status of a deployment, from what the backend runs of it.
	/// `Stopped` if it runs nothing of it.
	fn observe(
		&self,
		deployment_id: Uuid,
	) -> impl Future<Output = Result<ObservedDeploymentStatus, RunnerError>> + Send;

	/// A stream of owners whose objects changed on the backend, each queued to
	/// be reconciled and, for a deployment, observed. It should reconnect on its
	/// own: the stream ending means events may have been missed, so every owner
	/// is reconciled and the backend is watched again.
	fn watch(&self) -> impl Stream<Item = Owner> + Send + use<Self>;
}
