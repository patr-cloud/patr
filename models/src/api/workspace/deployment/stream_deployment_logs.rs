use time::OffsetDateTime;

use super::DeploymentLog;
use crate::prelude::*;

macros::declare_stream_endpoint!(
	/// Route to get the running logs of a deployment
	StreamDeploymentLogs,
	GET "/workspace/:workspace_id/deployment/:deployment_id/logs/stream" {
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
		/// The time from which the deployment logs should be fetched
		pub start_time: Option<OffsetDateTime>,
	},
	server_msg = {
		/// There is new log data for the deployment
		LogData {
			/// The deployment that was created
			#[serde(flatten)]
			logs: Vec<DeploymentLog>,
		},
	},
	client_msg = {},
);
