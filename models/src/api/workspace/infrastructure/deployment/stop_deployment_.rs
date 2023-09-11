use crate::prelude::*;

macros::declare_api_endpoint!(
    // Stop a deployment
    StopDeployment,
	POST "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/stop",
    path = {
        pub workspace_id: Uuid,
        pub deployment_id: Uuid,
    }
);
