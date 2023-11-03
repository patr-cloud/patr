use time::OffsetDateTime;

use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
};
use super::DeploymentLogs;

macros::declare_api_endpoint!(
    /// Route to get the running logs of a deployment
	GetDeploymentLog,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/logs" {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The deployment ID to get the logs for
        pub deployment_id: Uuid,
    },
    authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator { 
            extract_resource_id: |req| req.path.deployment_id
        }
	},
    request_headers = {
        /// Token used to authorize user
        pub authorization: BearerToken
    },
    query = {
        /// The time up until which the deployment logs should be fetched
        pub end_time: Option<OffsetDateTime>,
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
