use crate::prelude::*;

macros::declare_api_endpoint!(
    // Revert a deployment to a older digest
    RevertDeployment,
	POST "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/deploy-history/:digest/revert",
    path = {
        pub workspace_id: Uuid,
        pub deployment_id: Uuid,
        pub digest: String
    }
);
