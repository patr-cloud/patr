use crate::prelude::*;

macros::declare_api_endpoint!(
    // Get monitoring metrics for a deployment
	GetDeploymentMetrics,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/metrics",
    path = {
        pub workspace_id: Uuid,
        pub deployment_id: Uuid
    },
    request = {
        pub start_time: Option<Interval>,
        pub step: Option<Step>
    },
    response = {
        pub metrics: Vec<DeploymentMetrics>
    }
);
