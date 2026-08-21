use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::prelude::*;

/// The endpoint to accept a workspace invite
mod accept_workspace_invite;
/// All endpoints related to API tokens
mod api_token;
/// The endpoint to change the password of a user
mod change_password;
/// The endpoint to get the details of any user, based on their userId
mod get_user_details;
/// The endpoint to get the details of the currently logged in user
mod get_user_info;
/// The endpoint to list all the workspaces that a user is a part of
mod list_user_workspaces;
/// All endpoints related to MFA
mod mfa;
/// The endpoint to preview a workspace invite before accepting
mod preview_workspace_invite;
/// All endpoints related to social-login providers (list, disconnect, connect)
mod social_logins;
/// The endpoint to update the information of a user
mod update_user_info;
/// All endpoints related to web logins
mod web_logins;

pub use self::{
	accept_workspace_invite::*,
	api_token::*,
	change_password::*,
	get_user_details::*,
	get_user_info::*,
	list_user_workspaces::*,
	mfa::*,
	preview_workspace_invite::*,
	social_logins::*,
	update_user_info::*,
	web_logins::*,
};

/// This is the information that is _allowed_ to be public about a user.
///
/// This is not the entire user object, but only the information that is allowed
/// to be public. For privacy reasons, their email address — which is also their
/// unique identifier — is deliberately not part of this. Endpoints scoped to a
/// workspace expose co-members' emails separately, via
/// [`WorkspaceUserInfo`][crate::api::workspace::rbac::user::WorkspaceUserInfo].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ListableResource, TS)]
#[serde(rename_all = "camelCase")]
pub struct BasicUserInfo {
	/// The first name of the user.
	pub first_name: String,
	/// The last name of the user.
	pub last_name: String,
}
