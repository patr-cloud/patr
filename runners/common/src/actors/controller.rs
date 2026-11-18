use std::{
	collections::{BTreeMap, BTreeSet},
	marker::PhantomData,
	time::Duration,
};

use ractor::{Actor, ActorProcessingErr, ActorRef};
use sha2::{Digest as _, Sha256};
use tokio::task::JoinSet;

use super::{
	db_helpers,
	observer::{OBSERVER_NAME, ObserverMessage},
};
use crate::prelude::*;

/// The name the controller registers under, so other actors can find it.
pub const CONTROLLER_NAME: &str = "controller";

/// Bumped whenever how objects are applied changes, so every object's hash
/// changes and every object is applied again.
pub const SPEC_VERSION: u32 = 1;

/// How many owners reconcile at once.
const CONCURRENCY: usize = 4;

/// Initial delay before retrying an owner whose objects failed to apply.
const INITIAL_RETRY_BACKOFF: Duration = Duration::from_secs(1);

/// Maximum delay between retries of an owner whose objects fail to apply.
const MAX_RETRY_BACKOFF: Duration = Duration::from_secs(5 * 60);

/// Returns the interval between full reconciles of every owner. 10 seconds in
/// debug, 10 minutes in release.
fn full_sync_interval() -> Duration {
	if cfg!(debug_assertions) {
		Duration::from_secs(10)
	} else {
		Duration::from_secs(60 * 10)
	}
}

/// What reconciling one owner did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Outcome {
	/// An object failed to apply, or was blocked by a dependency that did. The
	/// owner is retried with backoff.
	pub failed: bool,
	/// An object couldn't be deleted yet: it's still in use, or deleting it
	/// failed. A later reconcile tries again.
	pub waiting: bool,
	/// An object was applied or deleted.
	pub changed: bool,
}

/// Messages for the [`Controller`].
pub enum ControllerMessage {
	/// These owners' objects may be out of date. Clears their backoff, since
	/// what they expect may have changed.
	Enqueue(BTreeSet<Owner>),
	/// A resync finished, so the local state is complete. Starts reconciling
	/// if it hadn't started.
	Start,
	/// Reconcile every owner, and schedule the next full reconcile.
	Reconcile,
	/// An owner's retry backoff elapsed.
	Retry(Owner),
	/// An owner's reconcile finished.
	Done {
		/// The owner that was reconciled.
		owner: Owner,
		/// What the reconcile did, or why it couldn't run.
		outcome: Result<Outcome, RunnerError>,
	},
}

/// Arguments passed to [`Controller::pre_start`].
pub struct ControllerArgs<E: RunnerExecutor> {
	/// The executor that applies and deletes objects.
	pub executor: E,
	/// Runner configuration.
	pub config: RunnerSettings<E::Settings>,
	/// Database connection pool for SQLite access.
	pub database: sqlx::Pool<DatabaseType>,
	/// Whether to reconcile before the first resync. Only a freshly created
	/// database waits, because until then it expects nothing and every object
	/// would be reaped.
	pub started: bool,
}

/// The mutable state held by a running [`Controller`].
pub struct ControllerState<E: RunnerExecutor> {
	/// The executor that applies and deletes objects.
	executor: E,
	/// Runner configuration.
	config: RunnerSettings<E::Settings>,
	/// Database connection pool for SQLite access.
	database: sqlx::Pool<DatabaseType>,
	/// Whether reconciling has started.
	started: bool,
	/// Owners waiting to be reconciled.
	pending: BTreeSet<Owner>,
	/// Owners being reconciled.
	in_flight: BTreeSet<Owner>,
	/// Owners enqueued while being reconciled, reconciled again once done.
	dirty: BTreeSet<Owner>,
	/// The current retry delay of each owner whose objects failed to apply.
	backoff: BTreeMap<Owner, Duration>,
	/// The running reconciles, aborted if the controller stops.
	tasks: JoinSet<()>,
}

/// Reconciles owners: makes the objects on the backend match what the local
/// state expects, one owner at a time per owner and a few owners at once.
pub struct Controller<E: RunnerExecutor> {
	/// Marker for the executor generic.
	_phantom: PhantomData<E>,
}

impl<E: RunnerExecutor> Controller<E> {
	/// Creates a new [`Controller`] instance.
	pub fn new() -> Self {
		Self {
			_phantom: PhantomData,
		}
	}
}

impl<E: RunnerExecutor> Actor for Controller<E> {
	type Arguments = ControllerArgs<E>;
	type Msg = ControllerMessage;
	type State = ControllerState<E>;

	async fn pre_start(
		&self,
		myself: ActorRef<Self::Msg>,
		args: Self::Arguments,
	) -> Result<Self::State, ActorProcessingErr> {
		let _ = myself.send_message(ControllerMessage::Reconcile);

		Ok(ControllerState {
			executor: args.executor,
			config: args.config,
			database: args.database,
			started: args.started,
			pending: BTreeSet::new(),
			in_flight: BTreeSet::new(),
			dirty: BTreeSet::new(),
			backoff: BTreeMap::new(),
			tasks: JoinSet::new(),
		})
	}

	async fn post_stop(
		&self,
		_: ActorRef<Self::Msg>,
		state: &mut Self::State,
	) -> Result<(), ActorProcessingErr> {
		state.tasks.abort_all();
		Ok(())
	}

	async fn handle(
		&self,
		myself: ActorRef<Self::Msg>,
		message: Self::Msg,
		state: &mut Self::State,
	) -> Result<(), ActorProcessingErr> {
		match message {
			ControllerMessage::Enqueue(owners) => {
				for owner in owners {
					state.backoff.remove(&owner);
					enqueue(state, owner);
				}
			}
			ControllerMessage::Start => {
				if !state.started {
					info!("Local state is complete, starting to reconcile");
					state.started = true;
					enqueue_all(state).await;
				}
			}
			ControllerMessage::Reconcile => {
				myself.send_after(full_sync_interval(), || ControllerMessage::Reconcile);
				if state.started {
					enqueue_all(state).await;
				}
			}
			ControllerMessage::Retry(owner) => {
				if state.backoff.contains_key(&owner) {
					enqueue(state, owner);
				}
			}
			ControllerMessage::Done { owner, outcome } => {
				state.in_flight.remove(&owner);
				while state.tasks.try_join_next().is_some() {}

				let failed = match &outcome {
					Ok(outcome) => outcome.failed,
					Err(err) => {
						error!(%owner, ?err, "Failed to reconcile");
						true
					}
				};

				if let Owner::Deployment(deployment_id) = owner &&
					let Some(observer) = ActorRef::<ObserverMessage>::where_is(OBSERVER_NAME)
				{
					let _ = observer.send_message(ObserverMessage::ApplyOutcome {
						deployment_id,
						failed,
					});
				}

				if failed {
					let delay = state
						.backoff
						.get(&owner)
						.map_or(INITIAL_RETRY_BACKOFF, |delay| {
							(*delay * 2).min(MAX_RETRY_BACKOFF)
						});
					state.backoff.insert(owner, delay);
					myself.send_after(delay, move || ControllerMessage::Retry(owner));
				} else {
					state.backoff.remove(&owner);
				}

				// The ingress mounts other owners' routes, so it follows their
				// changes.
				if owner != Owner::System && outcome.is_ok_and(|outcome| outcome.changed) {
					enqueue(state, Owner::System);
				}

				if state.dirty.remove(&owner) {
					state.pending.insert(owner);
				}
			}
		}

		if state.started {
			while state.in_flight.len() < CONCURRENCY {
				let Some(owner) = state
					.pending
					.take(&Owner::System)
					.or_else(|| state.pending.pop_first())
				else {
					break;
				};
				state.in_flight.insert(owner);

				let executor = state.executor.clone();
				let config = state.config.clone();
				let database = state.database.clone();
				let myself = myself.clone();
				state.tasks.spawn(async move {
					let outcome = reconcile(&executor, &config, &database, owner).await;
					let _ = myself.send_message(ControllerMessage::Done { owner, outcome });
				});
			}
		}

		Ok(())
	}
}

/// Queues an owner, or marks it to run again if it's being reconciled.
fn enqueue<E: RunnerExecutor>(state: &mut ControllerState<E>, owner: Owner) {
	if state.in_flight.contains(&owner) {
		state.dirty.insert(owner);
	} else {
		state.pending.insert(owner);
	}
}

/// Queues every owner: the runner itself, every deployment and managed URL in
/// the local state, and every owner objects exist for, so objects of owners
/// that are gone get reaped.
async fn enqueue_all<E: RunnerExecutor>(state: &mut ControllerState<E>) {
	let mut owners = BTreeSet::from([Owner::System]);

	match db_helpers::load_local_state(&state.database).await {
		Ok(local) => {
			owners.extend(local.deployments.keys().copied().map(Owner::Deployment));
			owners.extend(local.managed_urls.keys().copied().map(Owner::ManagedUrl));
		}
		Err(err) => error!(?err, "Failed to load the local state to reconcile"),
	}

	match state.executor.list(None).await {
		Ok(objects) => owners.extend(objects.into_iter().map(|object| object.owner)),
		Err(err) => error!(?err, "Failed to list objects to reconcile"),
	}

	for owner in owners {
		enqueue(state, owner);
	}
}

/// Reconciles one owner. Applies every expected object that's missing or out
/// of date, dependencies first, skipping objects whose dependencies failed.
/// Then deletes every object that's no longer expected, dependents first,
/// leaving objects that are still in use.
pub async fn reconcile<E: RunnerExecutor>(
	executor: &E,
	_config: &RunnerSettings<E::Settings>,
	database: &sqlx::Pool<DatabaseType>,
	owner: Owner,
) -> Result<Outcome, RunnerError> {
	let local = db_helpers::load_local_state(database).await?;
	let inputs = executor.inputs(owner).await?;
	let expected = executor.expected(owner, &local, &inputs)?;
	let actual = executor
		.list(Some(owner))
		.await?
		.into_iter()
		.map(|object| (object.id.clone(), object))
		.collect::<BTreeMap<_, _>>();

	let mut outcome = Outcome::default();

	// An object's hash covers its spec and its dependencies' hashes, so a
	// changed dependency re-applies everything that depends on it.
	let mut hashes = BTreeMap::<E::Id, String>::new();
	for id in expected.apply_order()? {
		let Some((spec, depends_on)) = expected.nodes.get(&id) else {
			continue;
		};

		let mut hasher = Sha256::new();
		hasher.update(SPEC_VERSION.to_be_bytes());
		hasher.update(serde_json::to_vec(spec).map_err(RunnerError::host)?);
		let mut blocked = false;
		for dependency in depends_on {
			let Some(dependency_hash) = hashes.get(dependency) else {
				blocked = true;
				break;
			};
			hasher.update(dependency.to_string());
			hasher.update(dependency_hash);
		}
		if blocked {
			outcome.failed = true;
			continue;
		}
		let hash = hasher
			.finalize()
			.iter()
			.map(|byte| format!("{byte:02x}"))
			.collect::<String>();

		if actual.get(&id).is_none_or(|object| object.hash != hash) {
			if let Err(err) = executor.apply(owner, &id, spec, &hash).await {
				warn!(%owner, %id, ?err, "Failed to apply object");
				outcome.failed = true;
				continue;
			}
			outcome.changed = true;
		}
		hashes.insert(id, hash);
	}

	let mut unexpected = ExpectedGraph::<E::Id, ()>::new(owner);
	for (id, object) in &actual {
		if expected.nodes.contains_key(id) {
			continue;
		}
		unexpected.add(
			id.clone(),
			(),
			object
				.depends_on
				.iter()
				.filter(|dependency| {
					actual.contains_key(*dependency) && !expected.nodes.contains_key(*dependency)
				})
				.cloned(),
		);
	}
	let mut reap_order = unexpected.apply_order()?;
	reap_order.reverse();

	for id in reap_order {
		match executor.in_use(&id).await {
			Ok(false) => {}
			Ok(true) => {
				outcome.waiting = true;
				continue;
			}
			Err(err) => {
				warn!(%owner, %id, ?err, "Failed to check whether an object is in use");
				outcome.waiting = true;
				continue;
			}
		}

		match executor.delete(&id).await {
			Ok(DeleteOutcome::Deleted) => outcome.changed = true,
			Ok(DeleteOutcome::InUse) => outcome.waiting = true,
			Err(err) => {
				warn!(%owner, %id, ?err, "Failed to delete object");
				outcome.waiting = true;
			}
		}
	}

	Ok(outcome)
}
