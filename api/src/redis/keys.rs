use std::net::IpAddr;

use models::api::auth::SocialLoginProvider;

use crate::prelude::*;

/// The key used to store the permissions for a login ID
pub fn permission_for_login_id(login_id: &Uuid) -> String {
	format!("permissions:{}", login_id)
}

/// The key used to store if a user ID has been revoked or not. If revoked,
/// any cached user data (that has been stored before the timestamp) will
/// have to be refetched from the database.
pub fn user_id_revocation_timestamp(user_id: &Uuid) -> String {
	format!("userIdRevocationTimestamp:{}", user_id)
}

/// The key used to store if a login ID has been revoked or not. If revoked,
/// any cached user data (that has been stored before the timestamp) will
/// have to be refetched from the database.
pub fn login_id_revocation_timestamp(login_id: &Uuid) -> String {
	format!("loginIdRevocationTimestamp:{}", login_id)
}

/// The key used to store if a workspace ID has been revoked or not. If revoked,
/// any cached user data (that has been stored before the timestamp) will
/// have to be refetched from the database.
pub fn workspace_id_revocation_timestamp(workspace_id: &Uuid) -> String {
	format!("workspaceIdRevocationTimestamp:{}", workspace_id)
}

/// The key used to store if all data has been revoked or not. If revoked,
/// any cached user data (that has been stored before the timestamp) will
/// have to be refetched from the database.
pub fn global_revocation_timestamp() -> String {
	String::from("globalRevocationTimestamp")
}

/// The key used to store the mfa secret of a user
pub fn user_mfa_secret(user_id: &Uuid) -> String {
	format!("mfa:{}", user_id)
}

/// The key used to store the Redis lock for a runner. This is used to ensure
/// that only one connection is allowed to stream data for a runner at a time,
/// and that the connection is not lost.
pub fn runner_connection_lock(runner_id: &Uuid) -> String {
	format!("{}{}", runner_connection_lock_prefix(), runner_id)
}

/// The prefix used for the runner connection lock key
pub fn runner_connection_lock_prefix() -> String {
	String::from("runnerConnectionLock:")
}

/// The prefix used for the current upload part and last byte of the multi-part
/// upload in the registry blob upload process
pub fn registry_blob_upload_part_prefix(session_id: &Uuid) -> String {
	format!("registryBlobUploadPart:{}", session_id)
}

/// The key used to store the pending buffer (bytes < 5MB that haven't been
/// flushed as an S3 part yet) for a chunked upload session. Stored as
/// base64-encoded data separate from the session object.
pub fn registry_blob_upload_pending_buffer(session_id: &Uuid) -> String {
	format!("registryBlobUploadPendingBuffer:{}", session_id)
}

/// The key used to temporarily associate a recently-uploaded blob with the
/// repository it was uploaded to. This is needed because between blob upload
/// and manifest push, the blob isn't yet linked to the repo via the manifest
/// tables. The key has the same TTL as the upload session (24h) and is deleted
/// once the manifest is pushed.
pub fn repository_for_registry_blob(repository_id: &Uuid, digest: &str) -> String {
	format!("repositoryForRegistryBlob:{}:{}", repository_id, digest)
}

/// The key used to cache the workspace ID that a runner belongs to. Cached
/// for 1 week; an empty value means the runner was deleted / not found.
pub fn workspace_id_for_runner(runner_id: &Uuid) -> String {
	format!("workspaceIdForRunner:{}", runner_id)
}

/// The key used to cache the runner ID that a deployment is assigned to. Cached
/// for 1 week; an empty value means the deployment was deleted / not found.
pub fn runner_id_for_deployment(deployment_id: &Uuid) -> String {
	format!("runnerIdForDeployment:{}", deployment_id)
}

/// The key holding a pending/live deployment-shell session, keyed by the
/// server-minted session ID. The value is a JSON
/// [`crate::models::ShellSession`] (`{workspace_id, runner_id,
/// deployment_id}`). Set with a short TTL for the dial-back window, then
/// refreshed by the CLI-facing handler as its liveness beacon while the session
/// runs — the runner-facing handler treats its absence as "the CLI-facing
/// instance died" and tears the session down.
pub fn shell_session(session_id: &Uuid) -> String {
	format!("shellSession:{}", session_id)
}

/// The runner-side liveness beacon for a deployment-shell session, refreshed by
/// the runner-facing handler once the runner has dialed back. The CLI-facing
/// handler treats its absence as "the runner-facing instance died".
pub fn shell_runner_alive(session_id: &Uuid) -> String {
	format!("shellRunnerAlive:{}", session_id)
}

/// The bounded byte-bridge list carrying frames from the CLI-facing handler to
/// the runner-facing handler (stdin / resize / close). The CLI-facing side
/// `LPUSH`es, the runner-facing side `RPOP`s.
pub fn shell_list_to_runner(session_id: &Uuid) -> String {
	format!("shell:{}:to-runner", session_id)
}

/// The bounded byte-bridge list carrying frames from the runner-facing handler
/// to the CLI-facing handler (connected / output / exit / error). The
/// runner-facing side `LPUSH`es, the CLI-facing side `RPOP`s; the runner-facing
/// producer gates on this list's `LLEN` for backpressure.
pub fn shell_list_to_client(session_id: &Uuid) -> String {
	format!("shell:{}:to-client", session_id)
}

/// The key used to store the IP lookup data for an IP address. This is used to
/// cache the results of IP lookups to avoid making repeated calls to the IPInfo
/// API for the same IP address, both to reduce latency and to reduce costs.
pub fn ip_lookup_data(ip: IpAddr) -> String {
	format!("ipLookupData:{}", ip)
}

/// The key used for the sliding window rate limiter sorted set, keyed by IP
/// address (or IPv6 /64 subnet) and window duration.
pub fn rate_limit_ip(identifier: &str, window_secs: u64) -> String {
	format!("rateLimit:ip:{}:{}", identifier, window_secs)
}

/// The key used for the sliding window rate limiter sorted set, keyed by login
/// ID and window duration. Used for per-login rate limiting on authenticated
/// endpoints.
pub fn rate_limit_login_id(login_id: &Uuid, window_secs: u64) -> String {
	format!("rateLimit:loginId:{}:{}", login_id, window_secs)
}

/// The key used to store a social-login OAuth CSRF state token. The value
/// is a JSON-serialised `GithubStatePayload` whose variant identifies
/// whether the token belongs to the unauthenticated sign-in flow or the
/// authenticated "Connect GitHub" flow. Expires after 10 minutes. Consumed
/// (deleted) on first use to prevent replay.
pub fn social_login_state(provider: &SocialLoginProvider, state_token: &str) -> String {
	format!("socialLogin:{}:state:{}", provider, state_token)
}

/// The key used to store a pending social-login account-setup payload for new
/// users. The value is JSON containing `{ external_id, email }`.
/// Expires after 10 minutes. Consumed on first use.
pub fn social_login_setup(provider: &SocialLoginProvider, setup_token: &str) -> String {
	format!("socialLogin:{}:setup:{}", provider, setup_token)
}
