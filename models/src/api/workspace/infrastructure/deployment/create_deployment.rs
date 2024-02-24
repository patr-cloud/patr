use super::{DeploymentRegistry, DeploymentRunningDetails};
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to create a new deployment
	CreateDeployment,
	POST "/workspace/:workspace_id/infrastructure/deployment" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id
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
		#[serde(flatten)]
		pub id: WithId<()>
	}
);
