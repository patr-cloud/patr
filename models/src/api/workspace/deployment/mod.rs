use std::{collections::BTreeMap, fmt::Display, str::FromStr};

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use time::OffsetDateTime;

/// The history of a deployment's deploys. This contains the image digest and
/// the timestamp of when the deploy was created
pub mod deploy_history;

/// The endpoint to create a deployment
mod create_deployment;
/// The endpoint to delete a deployment
mod delete_deployment;
/// The endpoint to get the details of a deployment
mod get_deployment_info;
/// The endpoint to get the logs of a deployment
mod get_deployment_logs;
/// The endpoint to get the metrics of a deployment
mod get_deployment_metric;
/// The endpoint to list all the machine types for deployments
mod list_all_deployment_machine_type;
/// The endpoint to list all the deployments in a workspace
mod list_deployment;
/// The endpoint to start a deployment
mod start_deployment;
/// The endpoint to stop a deployment
mod stop_deployment;
/// The endpoint to stream the logs of a deployment
mod stream_deployment_logs;
/// The endpoint to update a deployment's details
mod update_deployment;

pub use self::{
	create_deployment::*,
	delete_deployment::*,
	get_deployment_info::*,
	get_deployment_logs::*,
	get_deployment_metric::*,
	list_all_deployment_machine_type::*,
	list_deployment::*,
	start_deployment::*,
	stop_deployment::*,
	stream_deployment_logs::*,
	update_deployment::*,
};
use crate::{prelude::*, utils::constants};

/// The type of machine a deployment can run on.
///
/// This can be classified by the number of CPU and Memory allocated to the
/// deployment. The machine type can be used to classify the deployment based on
/// the resources it requires.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeploymentMachineType {
	/// The number of CPU nodes allocated to the deployment. This is the number
	/// of vCPUs in case of cloud deployments and the number of physical CPUs in
	/// case of on-prem deployments.
	pub cpu_count: u16,
	/// The amount of memory allocated to the deployment. This is the amount of
	/// RAM in 0.25 GB increments. So for a 4 GB machine, the memory count will
	/// be 16. This is the same for both cloud and on-prem deployments
	pub memory_count: u32,
}

/// Deployment information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
	/// Name of the deployment
	pub name: String,
	/// The registry of the image the deployment is running. This can be either
	/// the patr registry or an external registry. If it is the patr registry,
	/// the repository ID will be provided, else the registry URL and image name
	/// will be provided
	#[serde(flatten)]
	pub registry: DeploymentRegistry,
	/// The image tag of the deployment
	/// Example: 'latest', 'stable'
	pub image_tag: String,
	/// The status of the deployment
	pub status: DeploymentStatus,
	/// The runner the deployment is running on
	pub runner: Uuid,
	/// The deployment machine type ID
	/// Machine type can be classified by CPU and Memory nodes
	pub machine_type: Uuid,
	/// The current image digest the deployment is running
	pub current_live_digest: Option<String>,
}

/// Deployment running details
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DeploymentRunningDetails {
	/// if the deployment should deploy as soon as a new image digest is pushed
	pub deploy_on_push: bool,
	/// The minimum number node a deployment should maintain
	pub min_horizontal_scale: u16,
	/// The maximum number of node deployment can scale up to at peak resource
	/// requirement
	pub max_horizontal_scale: u16,
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	/// List of deployment port number of its type
	pub ports: BTreeMap<StringifiedU16, ExposedPortType>,
	/// List of environment variables are values
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub environment_variables: BTreeMap<String, EnvironmentVariableValue>,
	/// The startup probe of a deployment if any
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub startup_probe: Option<DeploymentProbe>,
	/// The liveness probe of a deployment if any
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub liveness_probe: Option<DeploymentProbe>,
	/// The config map attached to a deployment
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub config_mounts: BTreeMap<String, Base64String>,
	/// The volume ID attached to a deployment, along with the path it is
	/// mounted on
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub volumes: BTreeMap<Uuid, String>,
}

/// The type of environment variable
/// The keys can either have a string as a value or a secret
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[cfg_attr(not(target_arch = "wasm32"), derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum EnvironmentVariableValue {
	/// String
	String(String),
	/// Secret
	#[serde(rename_all = "camelCase")]
	Secret {
		/// The secret ID of the referred secret
		from_secret: Uuid,
	},
}

impl EnvironmentVariableValue {
	/// Check if the environment variable is a raw string
	pub fn is_string(&self) -> bool {
		matches!(self, Self::String { .. })
	}

	/// Check if the environment variable is a secret
	pub fn is_secret(&self) -> bool {
		matches!(self, Self::Secret { .. })
	}

	/// Get the secret ID
	pub fn secret_id(&self) -> Option<Uuid> {
		match self {
			Self::String(_) => None,
			Self::Secret { from_secret } => Some(*from_secret),
		}
	}

	/// Get the string value, if it is a raw string
	pub fn value(&self) -> Option<&String> {
		match self {
			Self::String(value) => Some(value),
			Self::Secret { .. } => None,
		}
	}
}

/// The type of exposed port
#[derive(
	Debug,
	Clone,
	Serialize,
	Deserialize,
	PartialEq,
	Eq,
	strum::EnumString,
	strum::Display,
	strum::VariantNames,
	Hash,
)]
#[strum(serialize_all = "camelCase")]
#[serde(rename_all = "camelCase")]
#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type, schemars::JsonSchema))]
#[cfg_attr(
	not(target_arch = "wasm32"),
	sqlx(type_name = "EXPOSED_PORT_TYPE", rename_all = "lowercase")
)]
pub enum ExposedPortType {
	/// TCP
	Tcp,
	/// UDP
	Udp,
	/// HTTP
	Http,
}

/// The deployment startup/liveness probe
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DeploymentProbe {
	/// The port the probe will be using
	pub port: u16,
	/// The path of the file to the probe commands
	pub path: String,
}

/// Patr registry
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), derive(schemars::JsonSchema))]
pub struct PatrRegistry;

impl Display for PatrRegistry {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", constants::CONTAINER_REGISTRY_URL)
	}
}

impl<'de> Deserialize<'de> for PatrRegistry {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let string = String::deserialize(deserializer)?;
		if string == constants::CONTAINER_REGISTRY_URL {
			Ok(PatrRegistry)
		} else {
			Err(Error::custom(format!(
				"str is not `{}`",
				constants::CONTAINER_REGISTRY_URL
			)))
		}
	}
}

impl Serialize for PatrRegistry {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(constants::CONTAINER_REGISTRY_URL)
	}
}

/// Deployment registry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum DeploymentRegistry {
	/// Patr registry offered by patr
	#[serde(rename_all = "camelCase")]
	PatrRegistry {
		/// Registry
		registry: PatrRegistry,
		/// Repo ID
		repository_id: Uuid,
	},
	/// Docker registry
	#[serde(rename_all = "camelCase")]
	ExternalRegistry {
		/// Registry
		registry: String,
		/// Image name
		image_name: String,
	},
}

impl DeploymentRegistry {
	/// Patr registry
	pub fn is_patr_registry(&self) -> bool {
		matches!(self, Self::PatrRegistry { .. })
	}

	/// External registry
	pub fn is_external_registry(&self) -> bool {
		matches!(self, Self::ExternalRegistry { .. })
	}

	/// Get the registry URL
	pub fn registry_url(&self) -> String {
		match self {
			Self::PatrRegistry { registry, .. } => format!("{registry}"),
			Self::ExternalRegistry { registry, .. } => registry.clone(),
		}
	}

	/// Get the registry's repository ID
	pub fn repository_id(&self) -> Option<Uuid> {
		match self {
			Self::PatrRegistry { repository_id, .. } => Some(*repository_id),
			Self::ExternalRegistry { .. } => None,
		}
	}

	/// Get the registry's image name
	pub fn image_name(&self) -> Option<String> {
		match self {
			Self::PatrRegistry { .. } => None,
			Self::ExternalRegistry { image_name, .. } => Some(image_name.clone()),
		}
	}
}

/// All the possible deployment status a deployment can be
/// in during its life cycle
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::Type, schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
	not(target_arch = "wasm32"),
	sqlx(type_name = "DEPLOYMENT_STATUS", rename_all = "lowercase")
)]
pub enum DeploymentStatus {
	/// Deployment has been created
	Created,
	/// Deployment is deploying
	Deploying,
	/// Deployment is running
	Running,
	/// Deployment has stopped
	Stopped,
	/// Deployment has errored and stopped
	Errored,
	/// The deployment's runner is not reachable
	Unreachable,
}

impl Display for DeploymentStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Created => write!(f, "created"),
			Self::Deploying => write!(f, "deploying"),
			Self::Running => write!(f, "running"),
			Self::Stopped => write!(f, "stopped"),
			Self::Errored => write!(f, "errored"),
			Self::Unreachable => write!(f, "unreachable"),
		}
	}
}

impl FromStr for DeploymentStatus {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.to_lowercase();
		match s.as_str() {
			"created" => Ok(Self::Created),
			"deploying" => Ok(Self::Deploying),
			"running" => Ok(Self::Running),
			"stopped" => Ok(Self::Stopped),
			"errored" => Ok(Self::Errored),
			"unreachable" => Ok(Self::Unreachable),
			_ => Err(s),
		}
	}
}

/// Deployment metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentMetric {
	/// The timestamp of the metric
	pub timestamp: OffsetDateTime,
	/// The cpu usage of a pod
	pub cpu_usage: String,
	/// The memory usage of a pod
	pub memory_usage: String,
	/// The network transmit of a pod
	pub network_usage_tx: String,
	/// The network recieve of a pod
	pub network_usage_rx: String,
}

/// Deployment logs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentLog {
	/// Timestamp of a deployment log
	pub timestamp: OffsetDateTime,
	/// The logs of a deployment
	pub log: String,
}
