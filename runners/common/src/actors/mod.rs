/// Shared database utility functions for creating and deleting deployments in
/// the local SQLite database. These are pure DB operations with no API calls or
/// runner configuration dependencies. Used by both the WebSocket actor
/// (managed mode server messages) and HTTP route handlers (self-hosted mode).
pub mod db_helpers;

/// The DeploymentActor manages the full lifecycle of a single deployment.
///
/// Each deployment gets its own actor instance, spawned as a supervised child
/// of the ResourceSupervisor. The actor reads desired state from SQLite,
/// compares it against the running state reported by the executor, and
/// reconciles any differences by calling `executor.upsert_deployment()` or
/// `executor.delete_deployment()`.
///
/// Status is polled periodically via a self-sent `CheckStatus` timer (5s).
/// This will be replaced by event-driven `StatusChanged` messages when the
/// Docker runner adds its event watcher in a future update.
pub mod deployment;

/// The ResourceSupervisor is the registry of all resource actors.
///
/// It replaces the old `Mutex<BTreeMap<Uuid, ResourceExecutorTask<E>>>` and the
/// `monitor_resources` SQLite trigger chain. It owns the mapping from resource
/// UUID to child actor, handles spawning and stopping child actors, runs
/// periodic reconciliation (comparing SQLite state against running actors),
/// and forwards status changes to the WebSocket actor for upstream
/// notification in managed mode.
///
/// When a child actor panics, ractor's supervision callback fires, the
/// supervisor removes the stale entry and queues a `Reconcile` to respawn it.
pub mod resource_supervisor;

/// The RunnerSupervisor is the top-level actor in the supervision tree.
///
/// It starts and supervises the ResourceSupervisor and WebSocketActor,
/// handling restarts when children fail. Currently a skeleton — the main
/// wiring happens in `Runner::run()` which spawns actors directly. This will
/// be fleshed out to own the full supervision tree in a future update.
pub mod runner_supervisor;

/// The WebSocketActor manages the bidirectional WebSocket connection to the
/// upstream Patr API. Only active in managed mode.
///
/// It receives server-pushed resource changes (deployment created, updated,
/// deleted), writes them to the local SQLite database, and notifies the
/// ResourceSupervisor to spawn or update the corresponding DeploymentActor.
/// In the reverse direction, it forwards deployment status updates from
/// resource actors back to the Patr API.
///
/// The read side uses `ractor_actors::streams::spawn_stream_pump` to feed
/// incoming WebSocket messages as `ServerMessage` events into the actor's
/// mailbox. The write side stores a type-erased boxed sink in actor state.
/// On connection failure, the actor schedules a reconnect with exponential
/// backoff. Periodic full resyncs re-fetch all deployments from the API to
/// catch any missed changes.
pub mod websocket;
