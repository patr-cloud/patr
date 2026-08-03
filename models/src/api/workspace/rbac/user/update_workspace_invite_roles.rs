use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to update the roles a pending invite will grant once accepted. This
	/// does not change the invite token or resend the email — the existing link
	/// stays valid and grants the updated roles.
	UpdateWorkspaceInviteRoles,
	PATCH "/workspace/{workspace_id}/rbac/user/invite/{invite_id}" {
		/// The ID of the workspace the invite belongs to
		pub workspace_id: Uuid,
		/// The ID of the invite to update
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
	request = {
		/// The new set of roles the invitee will be granted on acceptance
		#[preprocess(none)]
		pub roles: Vec<Uuid>,
	},
	audit_log = NoAuditLogger,
);
