use super::{Deployment, DeploymentRunningDetails};
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to get all the deployment information of a deployment
	GetDeploymentInfo,
	GET "/workspace/:workspace_id/deployment/:deployment_id" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID to get the event details for
		pub deployment_id: Uuid
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.deployment_id,
			permission: Permission::Deployment(DeploymentPermission::View),
		}
	},
	response = {
		/// The deployment metadata information containing:
		/// name - The deployment name
		/// registry - The deployment registry (patr registry or docker registry)
		/// image_tag - The deployment image tag
		/// region - The deployment region
		/// machine_type - The deployment machine type corresponding to CPU and RAM
		/// current_live_digest - The current live digest running
		#[serde(flatten)]
		pub deployment: WithId<Deployment>,
		/// The deployment details which contains information
		/// related to configuration containing:
		/// deploy_on_push - Is automatic update on new image push enabled
		/// min_horizontal_scale - The minimum number of pods to run
		/// max_horizontal_scale - The maximum number of pods to run
		/// port - The port the deployment will run on
		/// environment_variables - The environment variables set
		/// startup_probe - The startup probe configuration
		/// liveness_probe - The liveness probe configuration
		/// config_mounts - The configuration mounts
		/// volumes - The volumes
		#[serde(flatten)]
		pub running_details: DeploymentRunningDetails,
	}
);
