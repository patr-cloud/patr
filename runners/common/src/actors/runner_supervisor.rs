use std::marker::PhantomData;

use ractor::{Actor, ActorCell, ActorProcessingErr, ActorRef};

use super::{
	controller::{CONTROLLER_NAME, Controller, ControllerArgs},
	observer::{OBSERVER_NAME, Observer, ObserverArgs},
	websocket::{WEBSOCKET_NAME, WebSocketActor, WebSocketActorArgs},
};
use crate::prelude::*;

/// Arguments passed to [`RunnerSupervisor::pre_start`].
pub struct RunnerSupervisorArgs<E: RunnerExecutor> {
	/// Runner configuration.
	pub config: RunnerSettings<E::Settings>,
	/// Database connection pool for SQLite access.
	pub database: sqlx::Pool<DatabaseType>,
	/// The executor that runs resources on the backend.
	pub executor: E,
	/// Whether the controller can reconcile before the first resync, which is
	/// false only for a freshly created database.
	pub started: bool,
}

/// Top-level supervisor actor. Spawns the controller, the observer and the
/// WebSocket actor, and restarts whichever of them stops. They find each
/// other by their registered names, so a restarted actor is found again.
pub struct RunnerSupervisor<E: RunnerExecutor> {
	/// Marker for the executor generic.
	_phantom: PhantomData<E>,
}

impl<E: RunnerExecutor> RunnerSupervisor<E> {
	/// Creates a new [`RunnerSupervisor`] instance.
	pub fn new() -> Self {
		Self {
			_phantom: PhantomData,
		}
	}
}

impl<E: RunnerExecutor> Actor for RunnerSupervisor<E> {
	type Arguments = RunnerSupervisorArgs<E>;
	type Msg = ();
	type State = RunnerSupervisorArgs<E>;

	async fn pre_start(
		&self,
		myself: ActorRef<Self::Msg>,
		args: Self::Arguments,
	) -> Result<Self::State, ActorProcessingErr> {
		for name in [CONTROLLER_NAME, OBSERVER_NAME, WEBSOCKET_NAME] {
			spawn_child(name, &args, myself.get_cell()).await?;
		}
		Ok(args)
	}

	async fn handle_supervisor_evt(
		&self,
		myself: ActorRef<Self::Msg>,
		message: ractor::SupervisionEvent,
		state: &mut Self::State,
	) -> Result<(), ActorProcessingErr> {
		let cell = match &message {
			ractor::SupervisionEvent::ActorTerminated(cell, ..) => {
				warn!(
					actor_name = ?cell.get_name(),
					"Supervised child terminated cleanly, restarting"
				);
				cell
			}
			ractor::SupervisionEvent::ActorFailed(cell, err) => {
				error!(
					actor_name = ?cell.get_name(),
					?err,
					"Supervised child failed, restarting"
				);
				cell
			}
			_ => return Ok(()),
		};

		if let Some(name) = cell.get_name() {
			spawn_child(&name, state, myself.get_cell()).await?;
		}

		Ok(())
	}
}

/// Spawns the child actor registered under `name`, linked to the supervisor.
async fn spawn_child<E: RunnerExecutor>(
	name: &str,
	args: &RunnerSupervisorArgs<E>,
	supervisor: ActorCell,
) -> Result<(), ActorProcessingErr> {
	match name {
		CONTROLLER_NAME => {
			Controller::<E>::spawn_linked(
				Some(name.to_string()),
				Controller::new(),
				ControllerArgs {
					executor: args.executor.clone(),
					config: args.config.clone(),
					database: args.database.clone(),
					started: args.started,
				},
				supervisor,
			)
			.await?;
		}
		OBSERVER_NAME => {
			Observer::<E>::spawn_linked(
				Some(name.to_string()),
				Observer::new(),
				ObserverArgs {
					executor: args.executor.clone(),
					database: args.database.clone(),
				},
				supervisor,
			)
			.await?;
		}
		WEBSOCKET_NAME => {
			WebSocketActor::<E>::spawn_linked(
				Some(name.to_string()),
				WebSocketActor::new(),
				WebSocketActorArgs {
					config: args.config.clone(),
					database: args.database.clone(),
					executor: args.executor.clone(),
				},
				supervisor,
			)
			.await?;
		}
		_ => {}
	}
	Ok(())
}
