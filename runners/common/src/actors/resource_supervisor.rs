use std::{collections::BTreeMap, marker::PhantomData, time::Duration};

use models::{api::workspace::deployment::DeploymentStatus, rbac::ResourceType};
use ractor::{Actor, ActorProcessingErr, ActorRef, concurrency::JoinHandle};

use super::{
	deployment::{DeploymentActor, DeploymentActorArgs, DeploymentMessage},
	websocket::WebSocketMessage,
};
use crate::prelude::*;

/// Messages for the [`ResourceSupervisor`].
///
/// The ResourceSupervisor owns the mapping from resource ID to child actor and
/// handles reconciliation between SQLite state and running actors.
#[derive(Debug)]
pub enum ResourceSupervisorMessage {
	/// Create or ensure a resource actor exists. The actor reads its desired
	/// state from SQLite — this message is just a notification.
	UpsertResource {
		/// The UUID of the resource to create or update.
		resource_id: Uuid,
		/// The type of resource (deployment, database, etc.).
		// resource_type fields are not yet used — all resources are currently
		// deployments. They will be read when we add DatabaseActor, StaticSiteActor,
		// etc. and the supervisor dispatches by type.
		#[allow(dead_code)]
		resource_type: ResourceType,
	},
	/// Stop and remove a resource actor.
	DeleteResource {
		/// The UUID of the resource to delete.
		resource_id: Uuid,
		/// The type of resource being deleted.
		// resource_type fields are not yet used — all resources are currently
		// deployments. They will be read when we add DatabaseActor, StaticSiteActor,
		// etc. and the supervisor dispatches by type.
		#[allow(dead_code)]
		resource_type: ResourceType,
	},
	/// Full reconciliation: diff DB state vs running actors.
	/// Self-sent on startup, on a periodic timer, and after WS reconnect.
	Reconcile,
	/// A child resource actor reports a status change. The supervisor updates
	/// SQLite and forwards the update to the WebSocket actor (managed mode).
	ResourceStatusChanged {
		/// The UUID of the resource whose status changed.
		resource_id: Uuid,
		/// The type of resource reporting the status change.
		resource_type: ResourceType,
		/// The new status of the resource.
		status: DeploymentStatus,
	},
	/// Set or update the WebSocket actor reference. Sent after the WS actor
	/// starts, since it has a circular dependency (WS needs supervisor ref,
	/// supervisor needs WS ref).
	SetWebSocketRef(ActorRef<WebSocketMessage>),
}

/// Initial delay before respawning a failed child actor.
const INITIAL_RESPAWN_BACKOFF: Duration = Duration::from_secs(1);

/// Maximum delay between respawn attempts (5 minutes).
const MAX_RESPAWN_BACKOFF: Duration = Duration::from_secs(5 * 60);

/// Returns the interval between periodic full reconciliation runs.
/// 10 seconds in debug, 10 minutes in release.
fn full_sync_interval() -> Duration {
	if cfg!(debug_assertions) {
		Duration::from_secs(10)
	} else {
		Duration::from_secs(60 * 10)
	}
}

/// Arguments passed to [`ResourceSupervisor::pre_start`] to construct the
/// initial state.
pub struct ResourceSupervisorArgs<E: RunnerExecutor> {
	/// Database connection pool for SQLite access.
	pub database: sqlx::Pool<DatabaseType>,
	/// Runner configuration.
	pub config: RunnerSettings<E::Settings>,
	/// Executor-specific initialized state.
	pub runner_state: E::InitializedState,
	/// `None` in self-hosted mode (no upstream WebSocket).
	pub websocket_ref: Option<ActorRef<WebSocketMessage>>,
}

/// The mutable state held by a running [`ResourceSupervisor`].
pub struct ResourceSupervisorState<E: RunnerExecutor> {
	/// Database connection pool for SQLite access.
	pub database: sqlx::Pool<DatabaseType>,
	/// Runner configuration.
	pub config: RunnerSettings<E::Settings>,
	/// Executor-specific initialized state.
	pub runner_state: E::InitializedState,
	/// Reference to the WebSocket actor for forwarding status updates.
	/// `None` in self-hosted mode.
	pub websocket_ref: Option<ActorRef<WebSocketMessage>>,
	/// Maps resource UUID → (typed ActorRef, JoinHandle for cleanup).
	/// We maintain our own map because we need to look up children by
	/// deployment UUID, which ractor's built-in child tracking doesn't index.
	pub children: BTreeMap<Uuid, (ActorRef<DeploymentMessage>, JoinHandle<()>)>,
	/// Per-deployment exponential backoff for respawning failed actors.
	/// Doubled on each consecutive failure, capped at [`MAX_RESPAWN_BACKOFF`],
	/// and cleared on successful reconciliation or explicit upsert.
	pub respawn_backoff: BTreeMap<Uuid, Duration>,
}

/// Actor responsible for managing the registry of all resource actors.
///
/// Replaces the old `Mutex<BTreeMap<Uuid, ResourceExecutorTask<E>>>` and the
/// `monitor_resources` loop. Handles spawning/stopping child actors, periodic
/// reconciliation, and forwarding status changes to the WebSocket actor.
pub struct ResourceSupervisor<E: RunnerExecutor> {
	/// Marker for the executor generic.
	_phantom: PhantomData<E>,
}

impl<E: RunnerExecutor> ResourceSupervisor<E> {
	/// Creates a new [`ResourceSupervisor`] instance.
	pub fn new() -> Self {
		Self {
			_phantom: PhantomData,
		}
	}
}

impl<E> Actor for ResourceSupervisor<E>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	type Arguments = ResourceSupervisorArgs<E>;
	type Msg = ResourceSupervisorMessage;
	type State = ResourceSupervisorState<E>;

	async fn pre_start(
		&self,
		myself: ActorRef<Self::Msg>,
		args: Self::Arguments,
	) -> Result<Self::State, ActorProcessingErr> {
		// Queue an immediate reconciliation to sync with SQLite on startup.
		let _ = myself.send_message(ResourceSupervisorMessage::Reconcile);

		Ok(ResourceSupervisorState {
			database: args.database,
			config: args.config,
			runner_state: args.runner_state,
			websocket_ref: args.websocket_ref,
			children: BTreeMap::new(),
			respawn_backoff: BTreeMap::new(),
		})
	}

	async fn handle(
		&self,
		myself: ActorRef<Self::Msg>,
		message: Self::Msg,
		state: &mut Self::State,
	) -> Result<(), ActorProcessingErr> {
		match message {
			ResourceSupervisorMessage::UpsertResource {
				resource_id,
				resource_type,
			} => match resource_type {
				ResourceType::Deployment => {
					// If the actor is already running, clear any stale backoff and forward
					// ConfigUpdated. If not, only clear the backoff if this is an external
					// notification (the actor doesn't exist yet), which lets the spawn proceed. The
					// backoff timer also sends UpsertResource — in that case the child won't exist,
					// and clearing is fine since the timer already waited.
					if state.children.contains_key(&resource_id) {
						state.respawn_backoff.remove(&resource_id);
					}
					upsert_deployment_actor(&myself, state, resource_id).await?;
				}
				ResourceType::ManagedURL => {
					upsert_managed_url(state, resource_id).await?;
				}
				other => {
					warn!(?other, "Unsupported resource type in UpsertResource");
				}
			},
			ResourceSupervisorMessage::DeleteResource {
				resource_id,
				resource_type,
			} => match resource_type {
				ResourceType::Deployment => {
					delete_deployment_actor(state, resource_id).await;
				}
				ResourceType::ManagedURL => {
					if let Err(err) = delete_managed_url(state, resource_id).await {
						error!(
							managed_url_id = %resource_id,
							%err,
							"Failed to delete managed URL"
						);
					}
				}
				other => {
					warn!(?other, "Unsupported resource type in DeleteResource");
				}
			},
			ResourceSupervisorMessage::Reconcile => {
				reconcile_deployments(&myself, state).await?;
				reconcile_managed_urls(state).await?;

				// Schedule the next periodic reconciliation.
				myself.send_after(full_sync_interval(), || {
					ResourceSupervisorMessage::Reconcile
				});
			}
			ResourceSupervisorMessage::ResourceStatusChanged {
				resource_id,
				resource_type: _,
				status,
			} => {
				// Forward the status update to the WebSocket actor for upstream
				// notification (managed mode only).
				if let Some(ws_ref) = &state.websocket_ref {
					let _ = ws_ref.send_message(WebSocketMessage::SendStatusUpdate {
						resource_id,
						status,
					});
				}
			}
			ResourceSupervisorMessage::SetWebSocketRef(ws_ref) => {
				state.websocket_ref = Some(ws_ref);
			}
		}
		Ok(())
	}

	async fn handle_supervisor_evt(
		&self,
		myself: ActorRef<Self::Msg>,
		message: ractor::SupervisionEvent,
		state: &mut Self::State,
	) -> Result<(), ActorProcessingErr> {
		match message {
			ractor::SupervisionEvent::ActorTerminated(cell, _boxed_state, _reason) => {
				// Child actor stopped cleanly. Remove from our tracking map.
				let actor_id = cell.get_id();
				state
					.children
					.retain(|_, (ref_, _)| ref_.get_id() != actor_id);
			}
			ractor::SupervisionEvent::ActorFailed(cell, error) => {
				// Child actor failed. Find which deployment it was, remove
				// from tracking, and schedule a respawn with exponential
				// backoff to avoid a tight retry loop.
				let actor_id = cell.get_id();
				let failed_deployment_id = state
					.children
					.iter()
					.find(|(_, (ref_, _))| ref_.get_id() == actor_id)
					.map(|(id, _)| *id);
				state
					.children
					.retain(|_, (ref_, _)| ref_.get_id() != actor_id);

				if let Some(deployment_id) = failed_deployment_id {
					let backoff = state
						.respawn_backoff
						.get(&deployment_id)
						.map(|d| (*d * 2).min(MAX_RESPAWN_BACKOFF))
						.unwrap_or(INITIAL_RESPAWN_BACKOFF);
					state.respawn_backoff.insert(deployment_id, backoff);

					warn!(
						%actor_id,
						%deployment_id,
						?backoff,
						%error,
						"Child actor failed, scheduling respawn with backoff"
					);
					myself.send_after(backoff, move || ResourceSupervisorMessage::UpsertResource {
						resource_id: deployment_id,
						resource_type: ResourceType::Deployment,
					});
				} else {
					warn!(
						%actor_id,
						%error,
						"Unknown child actor failed"
					);
				}
			}
			_ => {}
		}
		Ok(())
	}
}

/// Ensure a DeploymentActor exists for the given deployment. If the actor
/// already exists, send it a `ConfigUpdated` notification. If not, spawn a
/// new actor as a supervised child.
async fn upsert_deployment_actor<E>(
	myself: &ActorRef<ResourceSupervisorMessage>,
	state: &mut ResourceSupervisorState<E>,
	deployment_id: Uuid,
) -> Result<(), ActorProcessingErr>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	if let Some((actor_ref, _)) = state.children.get(&deployment_id) {
		// Actor exists — send it a ConfigUpdated notification. If the send
		// fails (actor died between map lookup and send), that's fine:
		// ractor delivers supervision events before regular messages, so
		// handle_supervisor_evt will clean up the stale entry and queue a
		// Reconcile to respawn it. The new actor reads SQLite in pre_start.
		let _ = actor_ref.send_message(DeploymentMessage::ConfigUpdated);
	} else {
		// Spawn a new DeploymentActor as a supervised child.
		let actor_name = format!("deployment-{deployment_id}");
		let args = DeploymentActorArgs {
			deployment_id,
			database: state.database.clone(),
			config: state.config.clone(),
			runner_state: state.runner_state.clone(),
			supervisor_ref: myself.clone(),
		};

		let (actor_ref, handle) = DeploymentActor::<E>::spawn_linked(
			Some(actor_name),
			DeploymentActor::new(),
			args,
			myself.get_cell(),
		)
		.await?;

		state.children.insert(deployment_id, (actor_ref, handle));
	}
	Ok(())
}

/// Stop and remove the DeploymentActor for the given deployment, if it exists.
async fn delete_deployment_actor<E: RunnerExecutor>(
	state: &mut ResourceSupervisorState<E>,
	deployment_id: Uuid,
) {
	if let Some((actor_ref, _handle)) = state.children.remove(&deployment_id) {
		// Send Shutdown message so the actor can clean up and stop itself.
		let _ = actor_ref.send_message(DeploymentMessage::Shutdown);
	}
}

/// Reconcile all deployments: compare SQLite state against running child
/// actors, spawn missing actors and stop orphaned ones.
async fn reconcile_deployments<E>(
	myself: &ActorRef<ResourceSupervisorMessage>,
	state: &mut ResourceSupervisorState<E>,
) -> Result<(), ActorProcessingErr>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	// Get all deployment IDs from SQLite, sorted.
	let db_deployment_ids = query(
		r#"
		SELECT
			id
		FROM
			deployment
		WHERE
			deleted IS NULL
		ORDER BY
			id;
		"#,
	)
	.fetch_all(&state.database)
	.await?
	.into_iter()
	.filter_map(|row| row.try_get::<Uuid, _>("id").ok())
	.collect::<Vec<Uuid>>();

	// Get all running child actor IDs (our tracking map is already sorted
	// since it's a BTreeMap).
	let running_ids = state.children.keys().copied().collect::<Vec<Uuid>>();

	// Diff: spawn missing, stop orphaned, notify existing.
	let mut db_iter = db_deployment_ids.iter().peekable();
	let mut run_iter = running_ids.iter().peekable();

	let mut spawn_count = 0u32;
	let mut stop_count = 0u32;
	let mut notify_count = 0u32;

	loop {
		match (db_iter.peek(), run_iter.peek()) {
			(Some(&&db_id), Some(&&run_id)) => {
				use std::cmp::Ordering;
				match db_id.cmp(&run_id) {
					Ordering::Less => {
						// In DB but not running — spawn it, unless it's
						// backing off from a recent failure.
						if !state.respawn_backoff.contains_key(&db_id) {
							if let Err(err) = upsert_deployment_actor(myself, state, db_id).await {
								error!(
									deployment_id = %db_id,
									%err,
									"Failed to spawn DeploymentActor during reconciliation"
								);
							}
							spawn_count += 1;
						}
						db_iter.next();
					}
					Ordering::Greater => {
						// Running but not in DB — stop it.
						delete_deployment_actor(state, run_id).await;

						stop_count += 1;
						run_iter.next();
					}
					Ordering::Equal => {
						// Both — ensure it has fresh config. Send failure is
						// harmless: supervision handles dead children (see
						// comment in upsert_deployment_actor).
						if let Some((actor_ref, _)) = state.children.get(&db_id) {
							let _ = actor_ref.send_message(DeploymentMessage::ConfigUpdated);
						}

						notify_count += 1;
						db_iter.next();
						run_iter.next();
					}
				}
			}
			(Some(&&db_id), None) => {
				if !state.respawn_backoff.contains_key(&db_id) {
					if let Err(err) = upsert_deployment_actor(myself, state, db_id).await {
						error!(
							deployment_id = %db_id,
							%err,
							"Failed to spawn DeploymentActor during reconciliation"
						);
					}
					spawn_count += 1;
				}
				db_iter.next();
			}
			(None, Some(&&run_id)) => {
				delete_deployment_actor(state, run_id).await;

				stop_count += 1;
				run_iter.next();
			}
			(None, None) => break,
		}
	}

	debug!(
		spawned = spawn_count,
		stopped = stop_count,
		notified = notify_count,
		total = state.children.len(),
		"Reconciliation complete"
	);

	Ok(())
}

/// Read a managed URL row from SQLite and apply it via the runner executor.
/// Managed URLs are stateless config — no per-URL child actor — so this just
/// pokes the executor.
async fn upsert_managed_url<E>(
	state: &ResourceSupervisorState<E>,
	managed_url_id: Uuid,
) -> Result<(), RunnerError>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	let row = query(
		r#"
		SELECT
			host,
			path,
			deployment_id,
			port
		FROM
			managed_url
		WHERE
			id = $1;
		"#,
	)
	.bind(managed_url_id)
	.fetch_optional(&state.database)
	.await?;

	let Some(row) = row else {
		// Row was deleted between the upsert message and now — let the next
		// reconcile sweep clean up the executor side.
		return Ok(());
	};

	let host = row.try_get::<String, _>("host")?;
	let path = row.try_get::<String, _>("path")?;
	let deployment_id = row.try_get::<Uuid, _>("deployment_id")?;
	let port = row.try_get::<i64, _>("port")?;
	let port = u16::try_from(port)
		.map_err(|_| RunnerError::host(format!("managed_url.port out of range for u16: {port}")))?;

	let executor = E::new(&state.config, state.runner_state.clone()).await;
	executor
		.upsert_managed_url(managed_url_id, host, path, deployment_id, port)
		.await
}

/// Delete a managed URL via the runner executor.
async fn delete_managed_url<E>(
	state: &ResourceSupervisorState<E>,
	managed_url_id: Uuid,
) -> Result<(), RunnerError>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	let executor = E::new(&state.config, state.runner_state.clone()).await;
	executor.delete_managed_url(managed_url_id).await
}

/// Reconcile managed URLs: diff the IDs in SQLite against the IDs the
/// executor reports as currently configured, and call upsert/delete to
/// converge. Mirrors `reconcile_deployments`'s sorted-merge diff:
/// DB-only → upsert, running-only → delete, intersection → re-upsert so
/// content drift (e.g. a missed stream message before a full resync) is
/// caught. Re-upsert is cheap because `update_config` is content-hashed and
/// no-ops when the snippet is unchanged.
async fn reconcile_managed_urls<E>(state: &ResourceSupervisorState<E>) -> Result<(), RunnerError>
where
	E: RunnerExecutor + Send + Sync + 'static,
{
	let db_ids = query(
		r#"
		SELECT
			id
		FROM
			managed_url
		ORDER BY
			id;
		"#,
	)
	.fetch_all(&state.database)
	.await?
	.into_iter()
	.filter_map(|row| row.try_get::<Uuid, _>("id").ok())
	.collect::<Vec<Uuid>>();

	let executor = E::new(&state.config, state.runner_state.clone()).await;
	let mut running_ids = executor.list_running_managed_urls().await?;
	running_ids.sort();

	let mut db_iter = db_ids.iter().peekable();
	let mut run_iter = running_ids.iter().peekable();

	loop {
		match (db_iter.peek(), run_iter.peek()) {
			(Some(&&db_id), Some(&&run_id)) => {
				use std::cmp::Ordering;
				match db_id.cmp(&run_id) {
					Ordering::Less => {
						// In DB but not running — write the config.
						if let Err(err) = upsert_managed_url(state, db_id).await {
							error!(managed_url_id = %db_id, %err, "Failed to upsert managed URL during reconcile");
						}
						db_iter.next();
					}
					Ordering::Greater => {
						// Running but not in DB — drop the config.
						if let Err(err) = delete_managed_url(state, run_id).await {
							error!(managed_url_id = %run_id, %err, "Failed to delete managed URL during reconcile");
						}
						run_iter.next();
					}
					Ordering::Equal => {
						// Both — re-upsert to catch content drift. update_config
						// is content-hashed so this is a no-op when the snippet
						// hasn't changed.
						if let Err(err) = upsert_managed_url(state, db_id).await {
							error!(managed_url_id = %db_id, %err, "Failed to refresh managed URL during reconcile");
						}
						db_iter.next();
						run_iter.next();
					}
				}
			}
			(Some(&&db_id), None) => {
				if let Err(err) = upsert_managed_url(state, db_id).await {
					error!(managed_url_id = %db_id, %err, "Failed to upsert managed URL during reconcile");
				}
				db_iter.next();
			}
			(None, Some(&&run_id)) => {
				if let Err(err) = delete_managed_url(state, run_id).await {
					error!(managed_url_id = %run_id, %err, "Failed to delete managed URL during reconcile");
				}
				run_iter.next();
			}
			(None, None) => break,
		}
	}

	Ok(())
}
