use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use typed_builder::TypedBuilder;

use crate::{prelude::*, rbac::WorkspacePermission};

/// The type of identity that is making the authenticated request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "identityType")]
pub enum IdentityData {
	/// A human user.
	#[serde(rename_all = "camelCase")]
	User {
		/// The email address of the user. This is their unique identifier.
		email: String,
		/// The first name of the user.
		first_name: String,
		/// The last name of the user.
		last_name: String,
	},
	/// A service account (non-human identity for runners and automation).
	#[serde(rename_all = "camelCase")]
	ServiceAccount {
		/// The name of the service account.
		name: String,
	},
}

impl IdentityData {
	/// Returns the email address if this is a user identity.
	/// Returns `None` for service accounts.
	#[must_use]
	pub fn email(&self) -> Option<&str> {
		match self {
			Self::User { email, .. } => Some(email),
			Self::ServiceAccount { .. } => None,
		}
	}

	/// How to refer to this identity in text meant for a human — an email
	/// body, an audit entry, a notification.
	#[must_use]
	pub fn display_name(&self) -> String {
		match self {
			Self::User {
				first_name,
				last_name,
				..
			} => format!("{first_name} {last_name}"),
			Self::ServiceAccount { name } => name.clone(),
		}
	}
}

/// Represents the data of an identity that is used in an authenticated
/// endpoint. This can be either a user or a service account.
#[derive(Debug, Clone, Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(setter(into)))]
pub struct RequestUserData {
	/// The ID of the identity (user ID or service account ID).
	pub id: Uuid,
	/// The type-specific identity data.
	pub identity: IdentityData,
	/// When the identity was created.
	pub created: OffsetDateTime,
	/// The loginId of the current authenticated request.
	pub login_id: Uuid,
	/// The permissions that the identity has on all workspaces. This is a map
	/// of WorkspaceID -> What permissions the identity has on that workspace.
	pub permissions: BTreeMap<Uuid, WorkspacePermission>,
}

impl RequestUserData {
	/// Checks if the identity has the specified permission on the specified
	/// resource in the specified workspace.
	#[must_use]
	pub fn has_permission_on_resource(
		&self,
		workspace_id: Uuid,
		resource_id: Uuid,
		permission_id: Uuid,
	) -> bool {
		self.permissions.get(&workspace_id).is_some_and(|perms| {
			perms.has_permission_on_resource(workspace_id, resource_id, permission_id)
		})
	}
}
