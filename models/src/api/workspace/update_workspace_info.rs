use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to update a workspace's info based on the ID
	UpdateWorkspaceInfo,
	PATCH "/workspace/:workspace_id" {
		/// The ID of the workspace to update
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			// permission: Permissions::Workspace(WorkspacePermissions::UpdateInfo),
		}
	},
	request = {
		/// The new name of the workspace
		pub name: Option<String>,
	},
);