use std::collections::BTreeMap;

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
	/// The timestamp when the user's permissions were inserted into Redis
	pub creation_time: OffsetDateTime,
}
