use crate::prelude::*;

macros::declare_api_endpoint!(
    // Get build info of a deployment
	GetDeploymentBuildLogs,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/build-logs",
    path = {
        pub workspace_id: Uuid,
        pub deployment_id: Uuid
    },
	request = {
        pub start_time: Option<Interval>,
	},
    response = {
        pub logs: Vec<BuildLog>,
    }
);
