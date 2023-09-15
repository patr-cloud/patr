use crate::{
    prelude::*,
	utils::{DateTime, Uuid},
};
use chrono::Utc;

macros::declare_api_endpoint!(
    /// Route to get the running logs of a deployment
	GetDeploymentLogs,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/logs",
    request_headers = {
        /// Token used to authorize user
        pub access_token: AuthorizationToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The deployment ID to get the logs for
        pub deployment_id: Uuid
        /// The time up until which the deployment logs should be fetched
        pub end_time: Option<DateTime<Utc>>,
        /// The limit of logs to fetch
        pub limit: Option<u32>
    },
    response = {
        /// The deployment logs containing:
        /// timestamp - The timestamp of the log
        /// logs - The log message
        pub logs: Vec<DeploymentLogs>
    }
);
