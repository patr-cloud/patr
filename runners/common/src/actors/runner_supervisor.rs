use std::marker::PhantomData;

use models::rbac::ResourceType;
use ractor::{Actor, ActorProcessingErr, ActorRef, concurrency::JoinHandle};

use super::{
	resource_supervisor::{ResourceSupervisor, ResourceSupervisorArgs, ResourceSupervisorMessage},
	websocket::{WebSocketActor, WebSocketActorArgs},
};
use crate::prelude::*;

/// Messages for the [`RunnerSupervisor`].
///
/// These are the messages that external callers (HTTP routes) send. The
/// RunnerSupervisor forwards them to the ResourceSupervisor internally.
#[derive(Debug)]
pub enum RunnerSupervisorMessage {
	/// A resource was created or updated. Forward to ResourceSupervisor.
	UpsertResource {
		/// The UUID of the resource to create or update.
		resource_id: Uuid,
		/// The type of resource.
		resource_type: ResourceType,
	},
	/// A resource was deleted. Forward to ResourceSupervisor.
	DeleteResource {
		/// The UUID of the resource to delete.
		resource_id: Uuid,
		/// The type of resource being deleted.
		resource_type: ResourceType,
	},
}

/// Arguments passed to [`RunnerSupervisor::pre_start`].
pub struct RunnerSupervisorArgs<E: RunnerExecutor> {
	/// Runner configuration.
	pub config: RunnerSettings<E::Settings>,
	/// Database connection pool for SQLite access.
	pub database: sqlx::Pool<DatabaseType>,
	/// Executor-specific initialized state.
	pub runner_state: E::InitializedState,
}

/// The mutable state held by a running [`RunnerSupervisor`].
pub struct RunnerSupervisorState<E: RunnerExecutor> {
	/// Runner configuration.
	pub config: RunnerSettings<E::Settings>,
	/// Database connection pool for SQLite access.
	pub database: sqlx::Pool<DatabaseType>,
	/// Executor-specific initialized state.
	pub runner_state: E::InitializedState,
	/// Reference to the ResourceSupervisor child actor.
	pub supervisor_ref: ActorRef<ResourceSupervisorMessage>,
	/// Join handle for the ResourceSupervisor actor task.
	pub supervisor_handle: JoinHandle<()>,
	/// `None` in self-hosted mode.
	pub ws: Option<(ActorRef<super::websocket::WebSocketMessage>, JoinHandle<()>)>,
}

/// Top-level supervisor actor. Spawns and supervises the ResourceSupervisor
/// and WebSocketActor (managed mode). Restarts children on failure. HTTP
/// routes send messages here, which are forwarded to the ResourceSupervisor.
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

impl<E> Actor for RunnerSupervisor<E>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	type Arguments = RunnerSupervisorArgs<E>;
	type Msg = RunnerSupervisorMessage;
	type State = RunnerSupervisorState<E>;

	async fn pre_start(
		&self,
		myself: ActorRef<Self::Msg>,
		args: Self::Arguments,
	) -> Result<Self::State, ActorProcessingErr> {
		let (supervisor_ref, supervisor_handle) = ResourceSupervisor::<E>::spawn_linked(
			Some("resource-supervisor".to_string()),
			ResourceSupervisor::new(),
			ResourceSupervisorArgs {
				database: args.database.clone(),
				config: args.config.clone(),
				runner_state: args.runner_state.clone(),
				websocket_ref: None,
			},
			myself.get_cell(),
		)
		.await?;

		let ws = if args.config.mode.is_managed() {
			let (ws_ref, ws_handle) = WebSocketActor::<E>::spawn_linked(
				Some("websocket".to_string()),
				WebSocketActor::new(),
				WebSocketActorArgs {
					config: args.config.clone(),
					database: args.database.clone(),
					supervisor_ref: supervisor_ref.clone(),
					runner_state: args.runner_state.clone(),
				},
				myself.get_cell(),
			)
			.await?;

			let _ = supervisor_ref
				.send_message(ResourceSupervisorMessage::SetWebSocketRef(ws_ref.clone()));

			Some((ws_ref, ws_handle))
		} else {
			None
		};

		Ok(RunnerSupervisorState {
			config: args.config,
			database: args.database,
			runner_state: args.runner_state,
			supervisor_ref,
			supervisor_handle,
			ws,
		})
	}

	async fn handle(
		&self,
		_myself: ActorRef<Self::Msg>,
		message: Self::Msg,
		state: &mut Self::State,
	) -> Result<(), ActorProcessingErr> {
		// Forward to the ResourceSupervisor.
		let forwarded = match message {
			RunnerSupervisorMessage::UpsertResource {
				resource_id,
				resource_type,
			} => ResourceSupervisorMessage::UpsertResource {
				resource_id,
				resource_type,
			},
			RunnerSupervisorMessage::DeleteResource {
				resource_id,
				resource_type,
			} => ResourceSupervisorMessage::DeleteResource {
				resource_id,
				resource_type,
			},
		};
		let _ = state.supervisor_ref.send_message(forwarded);
		Ok(())
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
					actor_id = %cell.get_id(),
					actor_name = ?cell.get_name(),
					"Supervised child terminated cleanly, restarting"
				);
				cell.clone()
			}
			ractor::SupervisionEvent::ActorFailed(cell, err) => {
				error!(
					actor_id = %cell.get_id(),
					actor_name = ?cell.get_name(),
					?err,
					"Supervised child failed, restarting"
				);
				cell.clone()
			}
			_ => return Ok(()),
		};

		let actor_id = cell.get_id();

		if state.supervisor_ref.get_id() == actor_id {
			let (new_ref, new_handle) = ResourceSupervisor::<E>::spawn_linked(
				Some("resource-supervisor".to_string()),
				ResourceSupervisor::new(),
				ResourceSupervisorArgs {
					database: state.database.clone(),
					config: state.config.clone(),
					runner_state: state.runner_state.clone(),
					websocket_ref: state.ws.as_ref().map(|(ws_ref, _)| ws_ref.clone()),
				},
				myself.get_cell(),
			)
			.await?;

			state.supervisor_ref = new_ref;
			state.supervisor_handle = new_handle;
		} else if state
			.ws
			.as_ref()
			.is_some_and(|(ws_ref, _)| ws_ref.get_id() == actor_id)
		{
			let (new_ws_ref, new_ws_handle) = WebSocketActor::<E>::spawn_linked(
				Some("websocket".to_string()),
				WebSocketActor::new(),
				WebSocketActorArgs {
					config: state.config.clone(),
					database: state.database.clone(),
					supervisor_ref: state.supervisor_ref.clone(),
					runner_state: state.runner_state.clone(),
				},
				myself.get_cell(),
			)
			.await?;

			let _ = state
				.supervisor_ref
				.send_message(ResourceSupervisorMessage::SetWebSocketRef(
					new_ws_ref.clone(),
				));

			state.ws = Some((new_ws_ref, new_ws_handle));
		}

		Ok(())
	}
}
