use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
};
use super::DeploymentDeployHistory;

macros::declare_api_endpoint!(
    /// Route to get list of deployment history for a deployment
    ListDeploymentHistory,
	GET "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/deploy-history",
    request_headers = {
        /// Token used to authorize user
        pub access_token: BearerToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The deployment ID to get the history for
        pub deployment_id: Uuid
    },
    response = {
        /// The deployment history containing:
        /// image_digest - The image digest of the deployment
        /// created - The timestamp of when the deployment was created
        pub deploys: Vec<DeploymentDeployHistory>
    }
);
