use std::collections::{BTreeMap, BTreeSet};

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
	/// The email address of the user. This is their unique identifier.
	pub email: String,
	/// The first name of the user.
	pub first_name: String,
	/// The last name of the user.
	pub last_name: String,
	/// When the user account was created.
	pub created: OffsetDateTime,
	/// The loginId of the current authenticated request.
	pub login_id: Uuid,
	/// Every workspace this login belongs to (membership ∪ ownership for
	/// users; granted ∪ super-admin workspaces for API tokens). Independent
	/// of `permissions` — a member with zero bindings appears here and not
	/// there.
	pub workspaces: BTreeSet<Uuid>,
	/// The permissions that the user has on all workspaces. This is a map of
	/// WorkspaceID -> What permissions the user has on that workspace.
	pub permissions: BTreeMap<Uuid, WorkspacePermission>,
}

impl RequestUserData {
	/// Checks if the user has the specified permission on the specified
	/// resource in the specified workspace.
	#[must_use]
	pub fn has_permission_on_resource(
		&self,
		workspace_id: Uuid,
		resource_id: Uuid,
		permission_id: Uuid,
	) -> bool {
		self.permissions
			.get(&workspace_id)
			.is_some_and(|perms| perms.has_permission_on_resource(resource_id, permission_id))
	}
}
