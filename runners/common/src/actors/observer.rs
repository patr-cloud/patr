use std::{
	collections::{BTreeMap, BTreeSet},
	marker::PhantomData,
	time::Duration,
};

use models::api::workspace::deployment::*;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use ractor_actors::streams::spawn_stream_pump;

use super::{
	controller::{CONTROLLER_NAME, ControllerMessage},
	websocket::{WEBSOCKET_NAME, WebSocketMessage},
};
use crate::prelude::*;

/// The name the observer registers under, so other actors can find it.
pub const OBSERVER_NAME: &str = "observer";

/// How often the observer takes a status snapshot without a hint.
const OBSERVE_INTERVAL: Duration = Duration::from_secs(5);

/// How long the observer waits after a hint before taking a snapshot, so a
/// burst of backend events is observed once.
const HINT_DELAY: Duration = Duration::from_secs(1);

/// Messages for the [`Observer`].
pub enum ObserverMessage {
	/// Take a status snapshot, and schedule the next one.
	Tick,
	/// Take a status snapshot now.
	Observe,
	/// The outcome of a deployment's last reconcile. A deployment whose
	/// objects failed to apply is `Errored`, whatever is still running.
	ApplyOutcome {
		/// The deployment that was reconciled.
		deployment_id: Uuid,
		/// Whether its objects failed to apply.
		failed: bool,
	},
	/// An owner's objects changed on the backend.
	Hint(Owner),
}

/// Arguments passed to [`Observer::pre_start`].
pub struct ObserverArgs<E: RunnerExecutor> {
	/// The executor that observes the backend.
	pub executor: E,
	/// Database connection pool for SQLite access.
	pub database: sqlx::Pool<DatabaseType>,
}

/// The mutable state held by a running [`Observer`].
pub struct ObserverState<E: RunnerExecutor> {
	/// The executor that observes the backend.
	executor: E,
	/// Database connection pool for SQLite access.
	database: sqlx::Pool<DatabaseType>,
	/// The deployments whose last reconcile failed to apply.
	apply_failed: BTreeSet<Uuid>,
	/// Whether a snapshot after a hint is already scheduled.
	observe_scheduled: bool,
}

/// Works out each deployment's observed status from the backend and the
/// controller's last reconcile, records it in `deployment.observed_status`,
/// and tells the upstream to publish it.
pub struct Observer<E: RunnerExecutor> {
	/// Marker for the executor generic.
	_phantom: PhantomData<E>,
}

impl<E: RunnerExecutor> Observer<E> {
	/// Creates a new [`Observer`] instance.
	pub fn new() -> Self {
		Self {
			_phantom: PhantomData,
		}
	}
}

impl<E: RunnerExecutor> Actor for Observer<E> {
	type Arguments = ObserverArgs<E>;
	type Msg = ObserverMessage;
	type State = ObserverState<E>;

	async fn pre_start(
		&self,
		myself: ActorRef<Self::Msg>,
		args: Self::Arguments,
	) -> Result<Self::State, ActorProcessingErr> {
		// When the backend's event stream ends, the pump stops, which stops
		// this actor, and the supervisor restarts it with a fresh stream.
		spawn_stream_pump(
			args.executor.watch(),
			myself.clone(),
			|owner| match owner {
				Some(owner) => ObserverMessage::Hint(owner),
				None => ObserverMessage::Observe,
			},
			None,
		)
		.await?;

		let _ = myself.send_message(ObserverMessage::Tick);

		Ok(ObserverState {
			executor: args.executor,
			database: args.database,
			apply_failed: BTreeSet::new(),
			observe_scheduled: false,
		})
	}

	async fn handle(
		&self,
		myself: ActorRef<Self::Msg>,
		message: Self::Msg,
		state: &mut Self::State,
	) -> Result<(), ActorProcessingErr> {
		match message {
			ObserverMessage::Tick => {
				myself.send_after(OBSERVE_INTERVAL, || ObserverMessage::Tick);
			}
			ObserverMessage::Observe => {
				state.observe_scheduled = false;
			}
			ObserverMessage::ApplyOutcome {
				deployment_id,
				failed,
			} => {
				if failed {
					state.apply_failed.insert(deployment_id);
				} else {
					state.apply_failed.remove(&deployment_id);
				}
			}
			ObserverMessage::Hint(owner) => {
				if let Some(controller) = ActorRef::<ControllerMessage>::where_is(CONTROLLER_NAME) {
					let _ = controller
						.send_message(ControllerMessage::Enqueue(BTreeSet::from([owner])));
				}
				if !state.observe_scheduled {
					state.observe_scheduled = true;
					myself.send_after(HINT_DELAY, || ObserverMessage::Observe);
				}
				return Ok(());
			}
		}

		let observed = match state.executor.observe().await {
			Ok(observed) => observed,
			Err(err) => {
				warn!(?err, "Failed to observe deployments");
				return Ok(());
			}
		};

		let current = query!(
			r#"
			SELECT
				id,
				observed_status
			FROM
				deployment;
			"#
		)
		.fetch_all(&state.database)
		.await?
		.into_iter()
		.map(|row| (row.id, row.observed_status))
		.collect::<BTreeMap<_, _>>();

		state
			.apply_failed
			.retain(|deployment_id| current.contains_key(deployment_id));

		let websocket = ActorRef::<WebSocketMessage>::where_is(WEBSOCKET_NAME);
		for (deployment_id, current) in current {
			let status = if state.apply_failed.contains(&deployment_id) {
				ObservedDeploymentStatus::Errored
			} else {
				observed
					.get(&deployment_id)
					.copied()
					.unwrap_or(ObservedDeploymentStatus::Stopped)
			};
			if current == Some(status) {
				continue;
			}

			query!(
				r#"
				UPDATE
					deployment
				SET
					observed_status = $1
				WHERE
					id = $2;
				"#,
				status,
				deployment_id
			)
			.execute(&state.database)
			.await?;

			if let Some(websocket) = &websocket {
				let _ = websocket.send_message(WebSocketMessage::SendStatusUpdate {
					resource_id: deployment_id,
					status,
				});
			}
		}

		Ok(())
	}
}
