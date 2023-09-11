use crate::prelude::*;

macros::declare_api_endpoint!(
    // Start a deployment
    StartDeployment,
	POST "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/start",
    path = {
        pub workspace_id: Uuid,
        pub deployment_id: Uuid
    }
);
