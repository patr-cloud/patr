use std::collections::{BTreeMap, BTreeSet};

use crate::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestUserData {
    pub id: Uuid,
	pub username: String,
	pub first_name: String,
	pub last_name: String,
	pub created: OffsetDateTime,
	pub login_id: Uuid,
	pub permissions: BTreeMap<Uuid, WorkspacePermission>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkspacePermission {
	SuperAdmin,
	Member {
		// permission to resource mapping
		#[serde(flatten)]
		permissions: BTreeMap<Uuid, ResourcePermissionType>,
	},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResourcePermissionType {
	Include(
		// set of resource ids to allow
		BTreeSet<Uuid>,
	),
	Exclude(
		// set of resource ids to not allow
		BTreeSet<Uuid>,
	),
}


