use std::collections::BTreeMap;

use super::{DeploymentProbe, DeploymentVolume, EnvironmentVariableValue, ExposedPortType};
use crate::{
	prelude::*,
	utils::{Base64String, BearerToken, StringifiedU16, Uuid},
};

macros::declare_api_endpoint!(
	/// Route to update a deployment
	UpdateDeployment,
	PATCH "/workspace/:workspace_id/infrastructure/deployment/:deployment_id" {
		/// The workspace ID of the user
		pub workspace_id: Uuid,
		/// The deployment ID of the deployment to stop
		pub deployment_id: Uuid,
	},
	request_headers = {
		/// Token used to authorize user
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.deployment_id
		}
	},
	request = {
		/// To update the deployment name
		pub name: Option<String>,
		/// To update the machine type
		pub machine_type: Option<Uuid>,
		/// To update the automatic restart of deployment with new image once pushed
		pub deploy_on_push: Option<bool>,
		/// To update the minimum number of node
		pub min_horizontal_scale: Option<u16>,
		/// To update the maximum number of node
		pub max_horizontal_scale: Option<u16>,
		/// To update the ports
		pub ports: Option<BTreeMap<StringifiedU16, ExposedPortType>>,
		/// To update the environment variables
		pub environment_variables:
			Option<BTreeMap<String, EnvironmentVariableValue>>,
		/// To update the startup probe
		pub startup_probe: Option<DeploymentProbe>,
		/// To update the liveness probe
		pub liveness_probe: Option<DeploymentProbe>,
		/// To update the config mount
		pub config_mounts: Option<BTreeMap<String, Base64String>>,
		/// To update the volume size
		pub volumes: Option<BTreeMap<String, DeploymentVolume>>,
	}
);
