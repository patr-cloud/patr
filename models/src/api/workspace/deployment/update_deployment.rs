use std::collections::BTreeMap;

use super::{DeploymentProbe, EnvironmentVariableValue, ExposedPortType};
use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to update a deployment
	UpdateDeployment,
	PATCH "/workspace/:workspace_id/deployment/:deployment_id" {
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
			extract_resource_id: |req| req.path.deployment_id,
			permission: Permission::Deployment(DeploymentPermission::Edit),
		}
	},
	request = {
		/// To update the deployment name
		#[preprocess(optional(trim, regex = RESOURCE_NAME_REGEX))]
		pub name: Option<String>,
		/// Update which runner the deployment is running on
		#[preprocess(optional(none))]
		pub runner: Option<Uuid>,
		/// To update the machine type
		#[preprocess(none)]
		pub machine_type: Option<Uuid>,
		/// To update the automatic restart of deployment with new image once pushed
		#[preprocess(none)]
		pub deploy_on_push: Option<bool>,
		/// To update the minimum number of node
		#[preprocess(optional(range(min = 1)))]
		pub min_horizontal_scale: Option<u16>,
		/// To update the maximum number of node
		#[preprocess(optional(range(min = 1)))]
		pub max_horizontal_scale: Option<u16>,
		/// To update the ports
		#[preprocess(none)]
		pub ports: Option<BTreeMap<StringifiedU16, ExposedPortType>>,
		/// To update the environment variables
		#[preprocess(none)]
		pub environment_variables:
			Option<BTreeMap<String, EnvironmentVariableValue>>,
		/// To update the startup probe
		#[preprocess(none)]
		pub startup_probe: Option<DeploymentProbe>,
		/// To update the liveness probe
		#[preprocess(none)]
		pub liveness_probe: Option<DeploymentProbe>,
		/// To update the config mount
		#[preprocess(none)]
		pub config_mounts: Option<BTreeMap<String, Base64String>>,
		/// To update the volumes attached to the deployment
		#[preprocess(none)]
		pub volumes: Option<BTreeMap<Uuid, String>>,
	}
);

impl UpdateDeploymentRequest {
	/// Creates a new UpdateDeploymentRequest
	pub const fn new() -> Self {
		Self {
			name: None,
			ports: None,
			machine_type: None,
			deploy_on_push: None,
			min_horizontal_scale: None,
			max_horizontal_scale: None,
			environment_variables: None,
			liveness_probe: None,
			startup_probe: None,
			config_mounts: None,
			runner: None,
			volumes: None,
		}
	}

	/// Returns true if all fields are None
	pub fn is_none(&self) -> bool {
		self.name
			.as_ref()
			.map(|_| 0)
			.or(self.machine_type.as_ref().map(|_| 0))
			.or(self.deploy_on_push.as_ref().map(|_| 0))
			.or(self.runner.as_ref().map(|_| 0))
			.or(self.min_horizontal_scale.as_ref().map(|_| 0))
			.or(self.max_horizontal_scale.as_ref().map(|_| 0))
			.or(self.ports.as_ref().map(|_| 0))
			.or(self.environment_variables.as_ref().map(|_| 0))
			.or(self.startup_probe.as_ref().map(|_| 0))
			.or(self.liveness_probe.as_ref().map(|_| 0))
			.or(self.config_mounts.as_ref().map(|_| 0))
			.or(self.volumes.as_ref().map(|_| 0))
			.is_none()
	}
}

impl Default for UpdateDeploymentRequest {
	fn default() -> Self {
		Self::new()
	}
}
