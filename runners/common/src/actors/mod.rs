/// The Controller reconciles owners: for each one, it applies the objects the
/// local state expects and deletes the ones it no longer does.
///
/// Its queue holds owners whose objects may be out of date, never commands.
/// The WebSocketActor enqueues the owners an upstream change affects, the
/// Observer enqueues the owners backend events touch, and a full reconcile of
/// every owner runs on a timer, so nothing is lost if an event is missed.
pub mod controller;

/// Shared database utility functions for writing the local SQLite state and
/// loading it back. Used by the WebSocket actor to apply server messages, and
/// by the controller to load the state it reconciles.
pub mod db_helpers;

/// The Observer works out each deployment's observed status from the backend
/// and the controller's last reconcile, records it, and tells the WebSocket
/// actor to publish it.
pub mod observer;

/// The RunnerSupervisor is the top-level actor in the supervision tree.
///
/// It starts the Controller, the Observer and the WebSocketActor, and restarts
/// whichever stops. The actors find each other by their registered names.
pub mod runner_supervisor;

/// The WebSocketActor manages the bidirectional WebSocket connection to the
/// upstream Patr API.
///
/// It receives server-pushed resource changes, writes them to the local SQLite
/// database, and enqueues the owners they affect with the Controller. In the
/// reverse direction, it publishes the observed statuses the API doesn't
/// reflect yet.
///
/// The read side uses `ractor_actors::streams::spawn_stream_pump` to feed
/// incoming WebSocket messages as `ServerMessage` events into the actor's
/// mailbox. The write side stores a type-erased boxed sink in actor state.
/// On connection failure, the actor schedules a reconnect with exponential
/// backoff. Periodic full resyncs re-fetch everything from the API to catch any
/// missed changes.
pub mod websocket;
