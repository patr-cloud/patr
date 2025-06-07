use time::OffsetDateTime;

use super::DeploymentLog;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get the running logs of a deployment
	GetDeploymentLogs,
	GET "/workspace/:workspace_id/deployment/:deployment_id/logs" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID to get the logs for
		pub deployment_id: Uuid,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.deployment_id,
			permission: Permission::Deployment(DeploymentPermission::View),
		}
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	query = {
		/// The time up until which the deployment logs should be fetched
		pub end_time: Option<OffsetDateTime>,
		/// The limit of logs to fetch. Defaults to 100
		#[preprocess(range(max = Some(500)))]
		pub limit: Option<u32>,
		/// The search query to filter logs
		pub search: Option<String>,
	},
	response = {
		/// The deployment logs containing:
		/// timestamp - The timestamp of the log
		/// logs - The log message
		pub logs: Vec<DeploymentLog>
	}
);
