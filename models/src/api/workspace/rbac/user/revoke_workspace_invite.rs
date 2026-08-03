use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to revoke a pending workspace invite. Once revoked the invite link
	/// stops working.
	RevokeWorkspaceInvite,
	DELETE "/workspace/{workspace_id}/rbac/user/invite/{invite_id}" {
		/// The ID of the workspace the invite belongs to
		pub workspace_id: Uuid,
		/// The ID of the invite to revoke
		pub invite_id: Uuid,
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
			permission: Permission::ModifyRoles,
		}
	},
	audit_log = NoAuditLogger,
);
