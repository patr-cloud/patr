use crate::prelude::*;

macros::declare_api_endpoint!(
    // Get list of history for a deployment
    ListDeploymentHistory,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/deploy-history",
    path = {
        pub workspace_id: Uuid,
        pub deployment_id: Uuid
    },
    response = {
        pub deploys: Vec<DeploymentDeployHistory>
    }
);
