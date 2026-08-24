use std::collections::{BTreeMap, BTreeSet};

use models::rbac::WorkspacePermission;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::prelude::*;

/// The struct that is used to insert a user's permissions into Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPermissionCache {
	/// The user's permissions
	pub permission: BTreeMap<Uuid, WorkspacePermission>,
	/// Every workspace the login belongs to. No `serde(default)` on purpose:
	/// pre-existing cache entries without it fail to parse and fall through
	/// to a recompute.
	pub workspaces: BTreeSet<Uuid>,
	/// The timestamp when the user's permissions were inserted into Redis
	pub creation_time: OffsetDateTime,
}
