use crate::{
    prelude::*,
	utils::{Uuid, BearerToken},
};
use super::{DeploymentRegistry, DeploymentRunningDetails};

macros::declare_api_endpoint!(
    /// Route to create a new deployment
	CreateDeployment,
	POST "/workspace/:workspace_id/infrastructure/deployment" {
        /// The workspace ID of the user
        pub workspace_id: Uuid,
    },
    request_headers = {
        /// Token used to authorize user
        pub authorization: BearerToken
    },
    authentication = {
		AppAuthentication::<Self>::WorkspaceMembershipAuthenticator { 
            extract_workspace_id: |req| req.path.workspace_id 
        }
	},
	request = {
        /// The name of the deployment
		pub name: String,
        /// The registry the deployment will use
        /// It can either be patr's registry or docker's registry
        pub registry: DeploymentRegistry,
        /// The image tag to use
        pub image_tag: String,
        /// The region to deploy the deployment on
        pub region: Uuid,
        /// The machine type the deployment pod will run on
        /// Different machine types will have different resource allocation
        pub machine_type: Uuid,
        /// The details of the deployment which contains information related to configuration
        pub running_details: DeploymentRunningDetails,
        /// Option to start the deployment once it is created
        pub deploy_on_create: bool,
	},
	response = {
            /// The deployment ID of the created deployment
			pub id: Uuid,
	}
);
