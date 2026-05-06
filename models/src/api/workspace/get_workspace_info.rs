use crate::{api::workspace::Workspace, prelude::*};

macros::declare_api_endpoint!(
	/// Route to get a workspace's info based on the ID
	GetWorkspaceInfo,
	GET "",
	workspaced = true,
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
	response = {
		/// The details of the workspace requested
		#[serde(flatten)]
		pub workspace: WithId<Workspace>,
	},
	audit_log = NoAuditLogger,
);
