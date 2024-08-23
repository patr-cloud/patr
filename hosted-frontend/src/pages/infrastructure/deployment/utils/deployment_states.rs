use std::collections::BTreeMap;

use leptos::prelude::*;
use models::{
	api::workspace::deployment::{
		CreateDeploymentRequest,
		DeploymentProbe,
		DeploymentRegistry,
		DeploymentRunningDetails,
		EnvironmentVariableValue,
		ExposedPortType,
		PatrRegistry,
		*,
	},
	utils::{StringifiedU16, Uuid},
};

/// The State Data for deployment management page
#[derive(Debug, Clone)]
pub struct DeploymentInfoContext(pub RwSignal<Option<GetDeploymentInfoResponse>>);

/// The Deployment Info
#[derive(Clone, Debug)]
pub struct DeploymentInfo {
	/// The name of the deployment
	pub name: Option<String>,
	/// The Registry name of the deployment
	pub registry_name: Option<String>,
	/// The Image Tag of the deployment
	pub image_tag: Option<String>,
	/// The Image name of the deployment
	pub image_name: Option<String>,
	/// The Runner to use for the deployment
	pub runner_id: Option<Uuid>,
	/// The Machine Type to use for the deployment
	pub machine_type: Option<Uuid>,
	/// Whether to deploy on create
	pub deploy_on_create: bool,
	/// Whether to deploy on push
	pub deploy_on_push: bool,
	/// The minimum scale
	pub min_horizontal_scale: Option<u16>,
	/// The maximum scale
	pub max_horizontal_scale: Option<u16>,
	/// The ports to expose
	pub ports: BTreeMap<StringifiedU16, ExposedPortType>,
	/// The startup probe
	pub startup_probe: Option<(u16, String)>,
	/// The liveness probe
	pub liveness_probe: Option<(u16, String)>,
	/// The environment variables
	pub environment_variables: BTreeMap<String, EnvironmentVariableValue>,
	/// The volumes
	pub volumes: BTreeMap<Uuid, String>,
}

impl DeploymentInfo {
	/// Convert the Deployment Info to a CreateDeploymentRequest
	pub fn convert_to_deployment_req(&self) -> Option<CreateDeploymentRequest> {
		let image_name = self.image_name.clone()?;

		let registry = self.registry_name.clone()?;
		let registry = if registry.contains("patr") {
			let repository_id = Uuid::parse_str(image_name.as_str()).ok()?;
			DeploymentRegistry::PatrRegistry {
				repository_id,
				registry: PatrRegistry,
			}
		} else {
			DeploymentRegistry::ExternalRegistry {
				registry,
				image_name,
			}
		};

		let running_details = DeploymentRunningDetails {
			deploy_on_push: self.deploy_on_push,
			min_horizontal_scale: self.min_horizontal_scale.unwrap_or(1),
			max_horizontal_scale: self.max_horizontal_scale.unwrap_or(1),
			environment_variables: BTreeMap::from([]),
			ports: self.ports.clone(),
			liveness_probe: self
				.liveness_probe
				.clone()
				.map(|(port, path)| DeploymentProbe { port, path }),
			startup_probe: self
				.startup_probe
				.clone()
				.map(|(port, path)| DeploymentProbe { port, path }),
			volumes: self.volumes.clone(),
			config_mounts: BTreeMap::from([]),
		};

		Some(CreateDeploymentRequest {
			registry,
			running_details,
			name: self.name.clone()?,
			runner: self.runner_id.clone()?,
			image_tag: self.image_tag.clone()?,
			machine_type: self.machine_type.clone()?,
			deploy_on_create: self.deploy_on_create,
		})
	}
}

/// The Deployment Page user is on
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
	#[default]
	/// The Deployment Details Page
	Details,
	/// The Deployment Running Page
	Running,
	/// The Deployment Scaling Page
	Scaling,
}

impl Page {
	/// Get the next page
	pub fn next(&self) -> Self {
		match self {
			Self::Details => Self::Running,
			Self::Running => Self::Scaling,
			Self::Scaling => Self::Scaling,
		}
	}

	/// Get the previous page
	pub fn back(&self) -> Self {
		match self {
			Self::Scaling => Self::Running,
			Self::Running => Self::Details,
			Self::Details => Self::Details,
		}
	}
}
