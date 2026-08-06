use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to resend a pending workspace invite. This regenerates the invite
	/// token (invalidating the old link), refreshes the expiry, and sends the
	/// invite email again. The invited roles are left unchanged.
	ResendWorkspaceInvite,
	POST "/workspace/{workspace_id}/rbac/user/invite/{invite_id}/resend" {
		/// The ID of the workspace the invite belongs to
		pub workspace_id: Uuid,
		/// The ID of the invite to resend
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
	api = false,
	response = {
		/// The refreshed accept link for this invite, containing the new
		/// plaintext token, so the caller can offer a "copy link" affordance.
		pub accept_url: String,
	},
	audit_log = NoAuditLogger,
);
