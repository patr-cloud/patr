use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;

use crate::prelude::*;

/// A pending invite to join a workspace, as shown to admins on the members
/// page. The invite exists until the invitee accepts it or an admin revokes it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkspaceInvite {
	/// The email address the invite was sent to
	pub email: String,
	/// The role grants the invitee receives once they accept
	pub roles: Vec<super::RoleGrant>,
	/// The user who sent the invite
	pub invited_by: Uuid,
	/// When the invite was created
	#[ts(type = "Date")]
	pub created: OffsetDateTime,
	/// When the invite link expires
	#[ts(type = "Date")]
	pub expiry: OffsetDateTime,
}

macros::declare_api_endpoint!(
	/// Route to list all pending invites for a workspace.
	ListWorkspaceInvites,
	GET "/workspace/{workspace_id}/rbac/user/invite" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::ViewRoles,
		}
	},
	api = false,
	response = {
		/// The list of pending invites, each with its invite ID
		pub invites: Vec<WithId<WorkspaceInvite>>,
	},
	audit_log = NoAuditLogger,
);
