use std::net::IpAddr;

use models::api::auth::SocialLoginProvider;

use crate::prelude::*;

/// The key holding the cached authentication data for a login ID: the actor
/// behind it, which kind of login it is, and its permissions. Compared against
/// the `*_cache_stale_since` stamps below on every read.
pub fn auth_data_for_login_id(login_id: &Uuid) -> String {
	format!("authData:{}", login_id)
}

/// Stamp for one login ID: cached permissions for this login created before
/// this instant are stale and must be refetched from the database. Bumped when
/// the credential itself changes (revoked, regenerated, updated, logged out).
pub fn login_cache_stale_since(login_id: &Uuid) -> String {
	format!("loginCacheStaleSince:{}", login_id)
}

/// Stamp for one actor (user or service account): cached permissions for any
/// of its logins created before this instant are stale. Bumped when something
/// about the actor changes that every login inherits (password, MFA, roles,
/// workspace membership).
pub fn actor_cache_stale_since(actor_id: &Uuid) -> String {
	format!("actorCacheStaleSince:{}", actor_id)
}

/// Stamp for one workspace: cached permissions on this workspace created
/// before this instant are stale. Bumped when a role in the workspace changes
/// or the workspace is deleted.
pub fn workspace_cache_stale_since(workspace_id: &Uuid) -> String {
	format!("workspaceCacheStaleSince:{}", workspace_id)
}

/// Stamp for everything: all cached permissions created before this instant are
/// stale. Reserved for operator-driven global invalidation.
pub fn all_cache_stale_since() -> String {
	String::from("allCacheStaleSince")
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

/// The key used to store the IP lookup data for an IP address. This is used to
/// cache the results of IP lookups to avoid making repeated calls to the IPInfo
/// API for the same IP address, both to reduce latency and to reduce costs.
pub fn ip_lookup_data(ip: IpAddr) -> String {
	format!("ipLookupData:{}", ip)
}

/// The key used to store a pending runner consent link. Lookup by the
/// (workspace, 8-char user_code) tuple from the verification URL. Holds the
/// device_code (for constant-time compare on verify) plus the metadata the
/// consent page shows. TTL matches the link expiry (5 minutes). Approval
/// mutates the entry in place to add the issued runner+SA token; the entry
/// is deleted on first successful verify claim.
///
/// Workspace is part of the key so a verify/get/approve from a different
/// workspace's URL is naturally a key miss — no separate validation branch
/// in the handlers.
pub fn runner_setup_data(workspace_id: Uuid, user_code: &str) -> String {
	format!("runnerSetupData:{}:{}", workspace_id, user_code)
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
