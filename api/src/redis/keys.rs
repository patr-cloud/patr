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
