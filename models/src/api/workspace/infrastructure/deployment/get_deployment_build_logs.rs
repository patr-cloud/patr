use crate::{
    prelude::*,
	utils::Uuid,
};
use super::{BuildLog, Interval};

macros::declare_api_endpoint!(
    /// Route to get build logs of a deployment
	GetDeploymentBuildLogs,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/build-logs",
    request_headers = {
        /// Token used to authorize user
        pub access_token: AuthorizationToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The deployment ID to get build logs of
        pub deployment_id: Uuid
        /// The time from when the build logs should be fetched
        pub start_time: Option<Interval>,
	},
    response = {
        /// The deployment build logs which contains:
        /// timestamp - The timestamp of the log
        /// reason - The log type
        /// message - The log message
        pub logs: Vec<BuildLog>,
    }
);
