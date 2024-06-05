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
	/// The permissions that the user has on all workspaces.
	pub permissions: BTreeMap<Uuid, WorkspacePermission>,
}
