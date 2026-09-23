use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use typed_builder::TypedBuilder;

use crate::{prelude::*, rbac::WorkspacePermission, utils::ActorClientType};

/// How a user authenticated. Mirrors the database's `USER_LOGIN_TYPE`: a user
/// holds many logins, and each is either a web session or an API token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
	not(target_arch = "wasm32"),
	derive(sqlx::Type),
	sqlx(type_name = "USER_LOGIN_TYPE", rename_all = "snake_case")
)]
pub enum UserLoginType {
	/// A web dashboard session, authenticated via JWT.
	WebLogin,
	/// A user API token (`patrv1.*`).
	ApiToken,
}

/// The actor making the authenticated request. Mirrors the database's
/// `WORKSPACE_ACTOR_TYPE`: a human user, or a service account.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "actorType")]
pub enum ActorData {
	/// A human user.
	#[serde(rename_all = "camelCase")]
	User {
		/// The email address of the user. This is their unique identifier.
		email: String,
		/// The first name of the user.
		first_name: String,
		/// The last name of the user.
		last_name: String,
		/// Which of the user's logins this request came through.
		login: UserLoginType,
	},
	/// A service account (non-human identity for runners and automation). It
	/// holds a single credential, so there is nothing further to discriminate.
	#[serde(rename_all = "camelCase")]
	ServiceAccount {
		/// The name of the service account.
		name: String,
	},
}

impl ActorData {
	/// The kind of credential this actor authenticated with. Derived rather
	/// than stored, so an actor and its client type can never disagree.
	#[must_use]
	pub fn client_type(&self) -> ActorClientType {
		match self {
			Self::User { login, .. } => ActorClientType::UserLogin(*login),
			Self::ServiceAccount { .. } => ActorClientType::ServiceAccount,
		}
	}

	/// Returns the email address if this is a user.
	/// Returns `None` for service accounts.
	#[must_use]
	pub fn email(&self) -> Option<&str> {
		match self {
			Self::User { email, .. } => Some(email),
			Self::ServiceAccount { .. } => None,
		}
	}

	/// How to refer to this actor in text meant for a human — an email
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

/// Represents the data of an actor that is used in an authenticated
/// endpoint. This can be either a user or a service account.
#[derive(Debug, Clone, Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(setter(into)))]
pub struct RequestActorData {
	/// The ID of the actor (user ID or service account ID).
	pub id: Uuid,
	/// Who is making the request, and through which kind of credential.
	pub actor: ActorData,
	/// When the actor was created.
	pub created: OffsetDateTime,
	/// The loginId of the current authenticated request.
	pub login_id: Uuid,
	/// The permissions that the actor has on all workspaces. This is a map
	/// of WorkspaceID -> What permissions the actor has on that workspace.
	pub permissions: BTreeMap<Uuid, WorkspacePermission>,
}

impl RequestActorData {
	/// The kind of credential this request authenticated with.
	#[must_use]
	pub fn client_type(&self) -> ActorClientType {
		self.actor.client_type()
	}

	/// Checks if the actor has the specified permission on the specified
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
