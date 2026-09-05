use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::prelude::*;

/// The endpoint to invite a user, by email, to a workspace
mod invite_user_to_workspace;
/// The endpoint to list all the users in a workspace
mod list_users_in_workspace;
/// The endpoint to list all the pending invites for a workspace
mod list_workspace_invites;
/// The endpoint to remove a user from a workspace
mod remove_user_from_workspace;
/// The endpoint to resend a pending workspace invite
mod resend_workspace_invite;
/// The endpoint to revoke a pending workspace invite
mod revoke_workspace_invite;
/// The endpoint to update the roles of a user in a workspace
mod update_user_roles_in_workspace;
/// The endpoint to update the roles a pending invite will grant
mod update_workspace_invite_roles;

/// One role granted on a binding: the role plus the single resource it applies at. The only place a
/// permission target appears on the wire.
///
/// Granting the same role at several resources means several grants. The
/// workspace's own id is the root of the resource tree, so a grant there
/// applies to every resource in the workspace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, TS)]
#[serde(rename_all = "camelCase")]
pub struct RoleBindingGrant {
	/// The role being granted.
	pub role_id: Uuid,
	/// The resource the role applies at, or the workspace id for the whole
	/// workspace.
	pub resource_id: Uuid,
}

pub use self::{
	invite_user_to_workspace::*,
	list_users_in_workspace::*,
	list_workspace_invites::*,
	remove_user_from_workspace::*,
	resend_workspace_invite::*,
	revoke_workspace_invite::*,
	update_user_roles_in_workspace::*,
	update_workspace_invite_roles::*,
};

/// A member of a workspace, as seen by their co-members.
///
/// This is [`BasicUserInfo`][crate::api::user::BasicUserInfo] plus the member's
/// email address. Email is a user's unique identifier and is deliberately not
/// public, but sharing a workspace is the one relationship that makes it
/// visible: members need a stable, unambiguous handle for each other, and two
/// people can easily share a display name. Endpoints returning this are all
/// gated behind a workspace permission.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ListableResource, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceUserInfo {
	/// The first name of the member.
	pub first_name: String,
	/// The last name of the member.
	pub last_name: String,
	/// The email address of the member.
	pub email: String,
}
