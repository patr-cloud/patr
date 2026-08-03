//! Shared state and helpers for the interactive deployment-shell bridge.
//!
//! A shell session bridges two websockets that may land on different API
//! instances: the CLI-facing [`StreamDeploymentShell`] socket and the
//! runner-facing [`StreamRunnerShellConnection`] socket the runner dials back.
//! The two are relayed through a pair of bounded Redis Lists (point-to-point
//! with backpressure — see the module docs on the route handlers), plus a
//! per-side liveness beacon so an abruptly-dying API instance doesn't orphan
//! the other side.
//!
//! [`StreamDeploymentShell`]: models::api::workspace::deployment::StreamDeploymentShell
//! [`StreamRunnerShellConnection`]: models::api::workspace::runner::StreamRunnerShellConnection

use std::time::Duration;

use rustis::{
	client::Client as RedisClient,
	commands::{GenericCommands, ListCommands, StringCommands},
};
use serde::{Serialize, de::DeserializeOwned};

use crate::prelude::*;

/// TTL of the session record / CLI-facing liveness beacon. Short, since it's
/// refreshed every [`SHELL_BEACON_REFRESH`] while the session is alive.
pub const SHELL_SESSION_TTL_SECS: u64 = 15;

/// How often each side refreshes its liveness beacon.
pub const SHELL_BEACON_REFRESH: Duration = Duration::from_secs(5);

/// TTL applied to the byte-bridge lists as a leak backstop; refreshed on every
/// push so a live session never lets them expire.
pub const SHELL_LIST_TTL_SECS: u64 = 60;

/// How long the CLI-facing handler waits for the runner to dial back before
/// giving up. Shorter in debug builds so the timeout branch is quick to test.
pub const SHELL_DIAL_BACK_TIMEOUT: Duration = if cfg!(debug_assertions) {
	Duration::from_secs(3)
} else {
	Duration::from_secs(10)
};

/// The runner-facing producer stops reading the runner websocket once the
/// `to-client` list reaches this many buffered frames, and resumes below
/// [`SHELL_LIST_LOW_WATER`]. This is the backpressure gate — the stall
/// propagates via TCP all the way to the container.
pub const SHELL_LIST_HIGH_WATER: usize = 256;
/// Resume reading the runner websocket once the `to-client` list drains to
/// this.
pub const SHELL_LIST_LOW_WATER: usize = 64;

/// Poll interval floor while a bridge list is actively producing frames.
pub const SHELL_POLL_MIN: Duration = Duration::from_millis(2);
/// Poll interval ceiling once a bridge list has been idle for a while. Bounds
/// idle Redis load while keeping post-idle keystroke latency imperceptible.
pub const SHELL_POLL_MAX: Duration = Duration::from_millis(25);

/// The record stored under [`redis::keys::shell_session`] for the duration of a
/// shell session's handshake (and refreshed as the CLI-facing liveness beacon).
/// Binds a `session_id` to exactly one runner so a rogue runner can't attach to
/// someone else's session even if it learns the (unguessable) id.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ShellSession {
	/// The workspace the session belongs to.
	pub workspace_id: Uuid,
	/// The runner expected to service this session.
	pub runner_id: Uuid,
	/// The deployment the shell is opened into.
	pub deployment_id: Uuid,
}

/// `LPUSH` a JSON-serialised frame onto a bridge list and refresh the list's
/// TTL so a live session's lists never lapse.
pub async fn push_frame<T: Serialize>(
	redis: &RedisClient,
	key: &str,
	msg: &T,
) -> Result<(), ErrorType> {
	let payload = serde_json::to_string(msg)?;
	redis.lpush(key, payload).await?;
	redis.expire(key, SHELL_LIST_TTL_SECS, None).await?;
	Ok(())
}

/// `RPOP` a single JSON frame from a bridge list, deserialising it. Returns
/// `Ok(None)` when the list is empty.
pub async fn pop_frame<T: DeserializeOwned>(
	redis: &RedisClient,
	key: &str,
) -> Result<Option<T>, ErrorType> {
	let popped: Vec<String> = redis.rpop(key, 1).await?;
	let Some(raw) = popped.into_iter().next() else {
		return Ok(None);
	};
	Ok(Some(serde_json::from_str(&raw)?))
}

/// (Re)set a liveness beacon key with the standard TTL.
pub async fn refresh_beacon(redis: &RedisClient, key: &str) -> Result<(), ErrorType> {
	redis.setex(key, SHELL_SESSION_TTL_SECS, "1").await?;
	Ok(())
}

/// Whether a liveness beacon (or the session record) still exists — its absence
/// means the peer instance died.
pub async fn beacon_alive(redis: &RedisClient, key: &str) -> Result<bool, ErrorType> {
	let count: usize = redis.exists(key).await?;
	Ok(count > 0)
}

/// Best-effort deletion of every Redis key backing a session, on teardown.
pub async fn cleanup_session(redis: &RedisClient, session_id: &Uuid) {
	let keys = vec![
		redis::keys::shell_session(session_id),
		redis::keys::shell_runner_alive(session_id),
		redis::keys::shell_list_to_runner(session_id),
		redis::keys::shell_list_to_client(session_id),
	];
	let _: Result<usize, _> = redis.del(keys).await;
}
