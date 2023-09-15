use crate::{
    prelude::*,
	utils::Uuid,
}; 

macros::declare_api_endpoint!(
    /// Route to start a deployment
    StartDeployment,
	POST "/workspace/:workspace_id/infrastructure/deployment/:deployment_id/start",
    request_headers = {
        /// Token used to authorize user
        pub access_token: AuthorizationToken
    },
    query = {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
        /// The deployment ID of the deployment to start
        pub deployment_id: Uuid,
    },
);
