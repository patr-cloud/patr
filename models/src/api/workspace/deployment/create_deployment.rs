use super::{DeploymentRegistry, DeploymentRunningDetails};
use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to create a new deployment
	CreateDeployment,
	POST "/workspace/:workspace_id/deployment" {
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
			extract_resource_id: |req| req.path.workspace_id,
			permission: Permission::Deployment(DeploymentPermission::Create),
		}
	},
	request = {
		/// The name of the deployment
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
		/// The registry the deployment will use
		/// It can either be patr's registry or docker's registry
		#[preprocess(none)]
		#[serde(flatten)]
		pub registry: DeploymentRegistry,
		/// The image tag to use
		#[preprocess(trim, lowercase)]
		pub image_tag: String,
		/// The runner to use to run the deployment
		#[preprocess(none)]
		pub runner: Uuid,
		/// The machine type the deployment pod will run on
		/// Different machine types will have different resource allocation
		#[preprocess(none)]
		pub machine_type: Uuid,
		/// The details of the deployment which contains information related to configuration
		#[preprocess(none)]
		#[serde(flatten)]
		pub running_details: DeploymentRunningDetails,
		/// Option to start the deployment once it is created
		#[preprocess(none)]
		pub deploy_on_create: bool,
	},
	response = {
		/// The deployment ID of the created deployment
		#[serde(flatten)]
		pub id: WithId<()>,
	}
);
