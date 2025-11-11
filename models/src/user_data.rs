use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use typed_builder::TypedBuilder;

use crate::{prelude::*, rbac::WorkspacePermission};

/// Represents the data of a user that is used in an authenticated endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(setter(into)))]
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
	/// The permissions that the user has on all workspaces. This is a map of
	/// WorkspaceID -> What permissions the user has on that workspace.
	pub permissions: BTreeMap<Uuid, WorkspacePermission>,
}

impl RequestUserData {
	/// Check if the user has specific permission access to a given resource in
	/// a workspace. Returns true if the user has the required permission, false
	/// otherwise.
	#[must_use]
	pub fn has_resource_permission(
		&self,
		workspace_id: Uuid,
		resource_id: Uuid,
		required_permission: Uuid,
	) -> bool {
		self.permissions
			.get(&workspace_id)
			.map_or(false, |permission| {
				permission.has_resource_permission(resource_id, required_permission)
			})
	}
}
