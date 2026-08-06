use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route for the currently authenticated user to leave a workspace they are a
	/// member of. The owner (super admin) of a workspace cannot leave it.
	LeaveWorkspace,
	POST "/workspace/{workspace_id}/leave" {
		/// The ID of the workspace to leave
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::WorkspaceMembershipAuthenticator {
			extract_workspace_id: |req| req.path.workspace_id,
		}
	},
	api = false,
	audit_log = NoAuditLogger,
);
