use crate::{
    prelude::*,
	utils::Uuid,
}; 

macros::declare_api_endpoint!(
    /// Route to stop a deployment
    StopDeployment,
	POST "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/stop",
    request_headers = {
        /// Token used to authorize user
        pub access_token: AuthorizationToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The deployment ID of the deployment to stop
        pub deployment_id: Uuid,
    },
);
