/// The endpoint to invite a user, by email, to a workspace
mod invite_user_to_workspace;
/// The endpoint to list all the pending invites for a workspace
mod list_workspace_invites;
/// The endpoint to list all the users in a workspace
mod list_users_in_workspace;
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
