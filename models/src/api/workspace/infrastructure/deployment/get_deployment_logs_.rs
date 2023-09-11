use crate::prelude::*;

macros::declare_api_endpoint!(
    // Get logs for a deployment
	GetDeploymentLogs,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/logs",
    path = {
        pub workspace_id: Uuid,
        pub deployment_id: Uuid
    },
    request = {
        pub end_time: Option<DateTime<Utc>>,
        pub limit: Option<u32>
    },
    response = {
        pub logs: Vec<DeploymentLogs>
    }
);
