use crate::{
    prelude::*,
	utils::Uuid,
}; 
use super::Deployment;

macros::declare_api_endpoint!(
    /// Route to list all the deployments in a workspace
    ListDeployment,
	GET "/workspace/:workspace_id/infrastructure/deployment",
    request_headers = {
        /// Token used to authorize user
        pub access_token: AuthorizationToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
    },
    response = {
        /// The list of deployment in the workspace containing:
        /// id - The deployment ID
        /// name - The deployment name
        /// registry - The deployment registry (patr registry or docker registry)
        /// image_tag - The deployment image tag
        /// region - The deployment region
        /// machine_type - The deployment machine type corresponding to CPU and RAM
        /// current_live_digest - The current live digest running
        pub deployments: Vec<Deployment>,
    }
);
