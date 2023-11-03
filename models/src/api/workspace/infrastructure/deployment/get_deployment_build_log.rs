use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
};
use super::{BuildLog, Interval};

macros::declare_api_endpoint!(
    /// Route to get build logs of a deployment
	GetDeploymentBuildLog,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/build-logs" {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The deployment ID to get build logs of
        pub deployment_id: Uuid,
	},
    query = {
        /// The time from when the build logs should be fetched
        pub start_time: Option<Interval>,
    },    
    request_headers = {
        /// Token used to authorize user
        pub authorization: BearerToken
    },
    authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator { 
            extract_resource_id: |req| req.path.deployment_id
        }
	},
    response = {
        /// The deployment build logs which contains:
        /// timestamp - The timestamp of the log
        /// reason - The log type
        /// message - The log message
        pub logs: Vec<BuildLog>,
    }
);
