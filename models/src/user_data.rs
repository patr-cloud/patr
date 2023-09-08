use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::prelude::*;

/// Represents the data of a user that is used in an authenticated endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestUserData {
	/// The userId as per the database.
	pub id: Uuid,
	/// The username of the user.
	pub username: String,
	/// The first name of the user.
	pub first_name: String,
	/// The last name of the user.
	pub last_name: String,
	/// When the user account was created.
	pub created: OffsetDateTime,
	/// The loginId of the current authenticated request.
	pub login_id: Uuid,
	/// The permissions that the user has on all workspaces.
	pub permissions: BTreeMap<Uuid, WorkspacePermission>,
}

/// Represents the kind of permission that is granted on a workspace.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum WorkspacePermission {
	/// The user is the super admin of the workspace.
	SuperAdmin,
	/// The user is a member of the workspace.
	Member {
		/// List of Permission IDs and the type of permission that is granted.
		#[serde(flatten)]
		permissions: BTreeMap<Uuid, ResourcePermissionType>,
	},
}

/// Represents the type of permission that is granted on a set of Resource IDs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResourcePermissionType {
	/// The user is allowed to access a set of Resource IDs. Any other
	/// Resource IDs are by default not allowed.
	Include(
		/// Set of Resource IDs to allow
		BTreeSet<Uuid>,
	),
	/// The user is not allowed to access a set of Resource IDs. Any other
	/// Resource IDs are by default allowed.
	Exclude(
		/// Set of Resource IDs to not allow
		BTreeSet<Uuid>,
	),
}
