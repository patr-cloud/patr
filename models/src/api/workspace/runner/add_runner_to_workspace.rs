use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to add runner to a workspace
	AddRunnerToWorkspace,
	POST "/workspace/:workspace_id/runner" {
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
			extract_resource_id: |req| req.path.workspace_id
		}
	},
	request = {
		/// Name of the runner
		#[preprocess(lowercase)]
		pub name: String,
	},
	response = {
		/// The ID of the created runner
		#[serde(flatten)]
		pub id: WithId<()>
	}
);
