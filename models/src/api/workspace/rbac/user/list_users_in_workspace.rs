use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{RoleBindingGrant, WorkspaceUserInfo};
use crate::prelude::*;

/// A user in a workspace, along with the roles they hold there.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceMember {
	/// The member's ID, name, and email.
	#[serde(flatten)]
	pub user: WithId<WorkspaceUserInfo>,
	/// The role grants this member holds in the workspace — each a role and
	/// the scope it applies at. Empty for the owner, whose super-admin access
	/// doesn't come from a role.
	pub role_bindings: Vec<RoleBindingGrant>,
	/// Whether this member is the workspace's super-admin. The owner has
	/// implicit access to everything, so the UI hides the edit/remove
	/// controls for them.
	pub is_owner: bool,
}

macros::declare_api_endpoint!(
	/// Route to list all users and their role in a workspace
	ListUsersInWorkspace,
	GET "/workspace/{workspace_id}/rbac/user" {
		/// The ID of the workspace
		pub workspace_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	listable_resource = WorkspaceUserInfo,
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::ViewRoles,
		}
	},
	response_headers = {
		/// The total number of items in the pagination
		pub total_count: TotalCountHeader,
	},
	response = {
		/// All members of the workspace — including the owner — with their
		/// details and the role grants they hold.
		pub users: Vec<WorkspaceMember>,
	},
	client_type = [ApiToken, ServiceAccount, WebDashboard],
	audit_log = NoAuditLogger,
);
