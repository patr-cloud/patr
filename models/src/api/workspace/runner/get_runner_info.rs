use super::Runner;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get Runner information
	GetRunnerInfo,
	GET "/workspace/:workspace_id/runner/:runner_id" {
		/// The ID of the workspace
		pub workspace_id: Uuid,
		/// The runner ID
		pub runner_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.runner_id,
			permission: Permission::Runner(RunnerPermission::View),
		}
	},
	response = {
		/// The runner information
		pub runner: WithId<Runner>,
	}
);
