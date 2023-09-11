use crate::prelude::*;

macros::declare_api_endpoint!(
    // List all the deployments
    ListDeployment,
	GET "/workspace/:workspace_id/infrastructure/deployment",
    path = {
        pub workspace_id: Uuid
    },
    response = {
        pub deployments: Vec<Deployment>,
    }
);
