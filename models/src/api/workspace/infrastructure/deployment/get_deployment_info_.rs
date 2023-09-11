use crate::prelude::*;

macros::declare_api_endpoint!(
    // Get deployment info
	GetDeploymentInfo,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/",
    path = {
        pub workspace_id: Uuid,
        pub deployment_id: Uuid
    },
    response = {
        #[serde(flatten)]
        pub deployment: Deployment,
        #[serde(flatten)]
        pub running_details: DeploymentRunningDetails,
    }
);
