use std::{
	collections::{BTreeMap, BTreeSet},
	fmt::Display,
	panic::AssertUnwindSafe,
};

use ::time::OffsetDateTime;
use futures::{
	FutureExt,
	future::{self, Either},
};
use models::api::workspace::deployment::*;
use sha2::{Digest as _, Sha256};
use tokio::{
	sync::mpsc,
	task::JoinSet,
	time::{self, Duration},
};

use super::{db_helpers, observer::ObserverMessage};
use crate::prelude::*;

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
	/// An object couldn't be deleted yet: it's still in use, a finalizer
	/// hasn't finished, or deleting it failed. A later reconcile tries again.
	pub waiting: bool,
	/// An object was applied or deleted.
	pub changed: bool,
}

/// Messages for the controller.
pub enum ControllerMessage {
	/// These owners' objects may be out of date. Clears their backoff, since
	/// what they expect may have changed.
	Enqueue(BTreeSet<Owner>),
	/// Reconcile every owner now, instead of at the next full reconcile.
	ReconcileAll,
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

/// The controller's state, owned by [`run`].
struct ControllerState<E: RunnerExecutor> {
	/// The executor that applies and deletes objects.
	executor: E,
	/// Runner configuration.
	config: RunnerSettings<E::Settings>,
	/// Database connection pool for SQLite access.
	database: sqlx::Pool<DatabaseType>,
	/// Owners waiting to be reconciled.
	pending: BTreeSet<Owner>,
	/// Owners being reconciled.
	in_flight: BTreeSet<Owner>,
	/// Owners enqueued while being reconciled, reconciled again once done.
	dirty: BTreeSet<Owner>,
	/// The current retry delay of each owner whose objects failed to apply.
	backoff: BTreeMap<Owner, Duration>,
	/// The running reconciles and retry timers, aborted when the runner exits.
	tasks: JoinSet<()>,
}

/// Reconciles owners until the runner exits: makes the objects on the backend
/// match what the local state expects, one owner at a time per owner and a few
/// owners at once.
///
/// It reaps every object the local state doesn't expect, so it must never start
/// on a freshly created database before a resync has filled it.
pub async fn run<E: RunnerExecutor>(
	mut receiver: mpsc::UnboundedReceiver<ControllerMessage>,
	observer_tx: mpsc::UnboundedSender<ObserverMessage>,
	executor: E,
	config: RunnerSettings<E::Settings>,
	database: sqlx::Pool<DatabaseType>,
) -> Result<!, RunnerError> {
	// A finalizer still marked running was interrupted, since the reconciles
	// that run them stop with the runner.
	if let Err(err) = query!(
		r#"
		UPDATE
			object_finalizer
		SET
			state = 'pending'
		WHERE
			state = 'running';
		"#
	)
	.execute(&database)
	.await
	{
		error!(?err, "Failed to reset interrupted finalizers");
	}

	let mut state = ControllerState {
		executor,
		config,
		database,
		pending: BTreeSet::new(),
		in_flight: BTreeSet::new(),
		dirty: BTreeSet::new(),
		backoff: BTreeMap::new(),
		tasks: JoinSet::new(),
	};
	// The reconciles and retry timers report back on this.
	let (task_sender, mut task_receiver) = mpsc::unbounded_channel();

	// This is set to zero intentionally so that the first full reconcile
	// happens immediately, and then every full sync interval after that.
	let mut sleep_future = Box::pin(time::sleep(Duration::from_secs(0)));

	loop {
		let controller_future = future::select(
			sleep_future,
			future::select(Box::pin(receiver.recv()), Box::pin(task_receiver.recv())),
		)
		.with_cancel_check()
		.await?;
		match controller_future {
			Either::Left(((), _)) => {
				// Regularly (every 10 minutes in prod and 10 seconds in dev),
				// reconcile every owner, so nothing is lost if an event is missed.
				sleep_future = Box::pin(time::sleep(full_sync_interval()));
				enqueue_all(&mut state).await;
			}
			Either::Right((
				Either::Left((message, _)) | Either::Right((message, _)),
				pending_sleep,
			)) => {
				sleep_future = pending_sleep;
				let Some(message) = message else {
					// The controller message channel has been closed. This should not happen
					error!("Controller message channel has been closed");
					continue;
				};

				match message {
					ControllerMessage::Enqueue(owners) => {
						for owner in owners {
							state.backoff.remove(&owner);
							enqueue(&mut state, owner);
						}
					}
					ControllerMessage::ReconcileAll => {
						sleep_future = Box::pin(time::sleep(Duration::from_secs(0)));
					}
					ControllerMessage::Retry(owner) => {
						if state.backoff.contains_key(&owner) {
							enqueue(&mut state, owner);
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

						if let Owner::Deployment(deployment_id) = owner {
							let _ = observer_tx.send(ObserverMessage::ApplyOutcome {
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
							let task_sender = task_sender.clone();
							state.tasks.spawn(async move {
								time::sleep(delay).await;
								let _ = task_sender.send(ControllerMessage::Retry(owner));
							});
						} else {
							state.backoff.remove(&owner);
						}

						// The ingress mounts other owners' routes, so it follows
						// their changes.
						if owner != Owner::System && outcome.is_ok_and(|outcome| outcome.changed) {
							enqueue(&mut state, Owner::System);
						}

						if state.dirty.remove(&owner) {
							state.pending.insert(owner);
						}
					}
				}
			}
		}

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
			let task_sender = task_sender.clone();
			state.tasks.spawn(async move {
				// A reconcile that panics counts as a failed one, so its owner
				// isn't left in flight.
				let outcome = AssertUnwindSafe(reconcile(&executor, &config, &database, owner))
					.catch_unwind()
					.await
					.unwrap_or_else(|_| Err(RunnerError::host("the reconcile panicked")));
				let _ = task_sender.send(ControllerMessage::Done { owner, outcome });
			});
		}
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
/// leaving objects that are still in use or whose finalizers haven't finished.
pub async fn reconcile<E: RunnerExecutor>(
	executor: &E,
	config: &RunnerSettings<E::Settings>,
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

		let finalizers = executor.finalizers(&id);
		if !finalizers.is_empty() {
			// An object with finalizers lives as long as its owner: a volume
			// a deployment no longer mounts keeps its data until the
			// deployment itself is gone.
			if !expected.nodes.is_empty() {
				continue;
			}

			let mut finalized = true;
			for finalizer in finalizers {
				match finalize(config, database, owner, &id, finalizer).await {
					Ok(done) => finalized &= done,
					Err(err) => {
						warn!(%owner, %id, %finalizer, ?err, "Failed to run finalizer");
						finalized = false;
					}
				}
			}
			if !finalized {
				outcome.waiting = true;
				continue;
			}
		}

		match executor.delete(&id).await {
			Ok(DeleteOutcome::Deleted) => {
				outcome.changed = true;
				query!(
					r#"
					DELETE FROM
						object_finalizer
					WHERE
						object_id = $1;
					"#,
					id.to_string()
				)
				.execute(database)
				.await?;
			}
			Ok(DeleteOutcome::InUse) => outcome.waiting = true,
			Err(err) => {
				warn!(%owner, %id, ?err, "Failed to delete object");
				outcome.waiting = true;
			}
		}
	}

	Ok(outcome)
}

/// Runs one finalizer of an object, recording its progress in
/// `object_finalizer`. Returns whether it's done. A finalizer that's already
/// done isn't run again.
async fn finalize<S>(
	config: &RunnerSettings<S>,
	database: &sqlx::Pool<DatabaseType>,
	owner: Owner,
	object_id: &impl Display,
	finalizer: FinalizerKind,
) -> Result<bool, RunnerError> {
	let object_id = object_id.to_string();

	let state = query!(
		r#"
		SELECT
			state
		FROM
			object_finalizer
		WHERE
			object_id = $1 AND
			finalizer = $2;
		"#,
		object_id,
		finalizer.to_string()
	)
	.fetch_optional(database)
	.await?
	.map(|row| row.state);
	if state.as_deref() == Some("done") {
		return Ok(true);
	}

	let Owner::Deployment(deployment_id) = owner else {
		return Err(RunnerError::host(format!(
			"`{object_id}` has a {finalizer} finalizer, but `{owner}` isn't a deployment"
		)));
	};

	query!(
		r#"
		INSERT INTO
			object_finalizer(
				object_id,
				deployment_id,
				finalizer,
				state,
				attempts,
				last_error,
				last_updated
			)
		VALUES
			($1, $2, $3, 'running', 1, NULL, $4)
		ON CONFLICT(object_id, finalizer) DO UPDATE SET
			state = 'running',
			attempts = attempts + 1,
			last_updated = $4;
		"#,
		object_id,
		deployment_id,
		finalizer.to_string(),
		OffsetDateTime::now_utc()
	)
	.execute(database)
	.await?;

	let last_error = match finalizer {
		FinalizerKind::Volume => {
			let RunnerSettings {
				workspace_id,
				runner_id,
				api_token,
				user_agent,
				..
			} = config;

			// The API answers the same 401 for a deleted deployment as for
			// one the runner can't see, and both mean the volume's data
			// isn't needed here any more.
			match client::make_request(
				ApiRequest::<GetDeploymentInfoRequest>::builder()
					.path(GetDeploymentInfoPath {
						workspace_id: *workspace_id,
						deployment_id,
					})
					.query(())
					.headers(GetDeploymentInfoRequestHeaders {
						authorization: api_token.clone(),
						user_agent: user_agent.clone(),
					})
					.body(GetDeploymentInfoRequest)
					.build(),
			)
			.await
			{
				Ok(response) if response.body.deployment.runner != *runner_id => None,
				Ok(_) => Some("still_on_this_runner"),
				Err(err)
					if matches!(
						err.body.error,
						ErrorType::Unauthorized | ErrorType::ResourceDoesNotExist
					) =>
				{
					None
				}
				Err(_) => Some("api_unreachable"),
			}
		}
	};

	query!(
		r#"
		UPDATE
			object_finalizer
		SET
			state = $1,
			last_error = $2,
			last_updated = $3
		WHERE
			object_id = $4 AND
			finalizer = $5;
		"#,
		if last_error.is_none() {
			"done"
		} else {
			"failed"
		},
		last_error,
		OffsetDateTime::now_utc(),
		object_id,
		finalizer.to_string()
	)
	.execute(database)
	.await?;

	Ok(last_error.is_none())
}
