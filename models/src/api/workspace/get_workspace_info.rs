use crate::{api::workspace::Workspace, prelude::*, utils::BearerToken};

macros::declare_api_endpoint!(
	/// Route to get a workspace's info based on the ID
	GetWorkspaceInfo,
	GET "/workspace/:workspace_id" {
		/// The ID of the workspace to get the info of
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
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
	}
);
