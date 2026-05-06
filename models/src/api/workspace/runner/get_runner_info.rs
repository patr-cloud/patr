use super::Runner;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get Runner information
	GetRunnerInfo,
	GET "/runner/{runner_id}" {
		/// The runner ID
		pub runner_id: Uuid,
	},
	workspaced = true,
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.runner_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Runner(RunnerPermission::View),
		}
	},
	response = {
		/// The runner information
		pub runner: WithId<Runner>,
	},
	audit_log = NoAuditLogger,
);
