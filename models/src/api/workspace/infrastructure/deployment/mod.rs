use std::{collections::BTreeMap, fmt::Display, str::FromStr};

use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use time::OffsetDateTime;

mod create_deployment;
mod delete_deployment;
mod get_deployment_build_log;
mod get_deployment_event;
mod get_deployment_info;
mod get_deployment_log;
mod get_deployment_metric;
mod list_deployment_history;
mod list_deployment;
mod list_linked_url;
mod revert_deployment;
mod start_deployment;
mod stop_deployment;
mod update_deployment;
mod list_all_deployment_machine_type;

pub use self::{
	create_deployment::*,
	delete_deployment::*,
	get_deployment_build_log::*,
	get_deployment_event::*,
	get_deployment_info::*,
	get_deployment_log::*,
	get_deployment_metric::*,
	list_deployment_history::*,
	list_deployment::*,
	list_linked_url::*,
	revert_deployment::*,
	start_deployment::*,
	stop_deployment::*,
	update_deployment::*,
	list_all_deployment_machine_type::*,
};
use crate::prelude::*;

/// Information of all the different deployment plans currently supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentMachineType {
	/// The number of CPU nodes
	pub cpu_count: i16,
	/// The number of memory nodes
	pub memory_count: i32,
}


/// Deployment information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
	/// Name of the deployment
	pub name: String,
	/// The registy of which the image is running
	/// Can either be patr registry or docker registry
	#[serde(flatten)]
	pub registry: DeploymentRegistry,
	/// The image tag 
	/// Example: 'latest', 'stable'
	pub image_tag: String,
	/// The status of the deployment
	pub status: DeploymentStatus,
	/// The region the deployment is running on
	pub region: Uuid,
	/// The deployment machine type ID
	/// Machine type can be classified by CPU and Memory nodes
	pub machine_type: Uuid,
	/// The current image digest the deployment is running 
	pub current_live_digest: Option<String>,
}

/// Deployment history
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentDeployHistory {
	/// The images digests the deployment has ran
	pub image_digest: Vec<String>,
	/// The timestamp of when the digest previously ran
	pub created: OffsetDateTime,
}

/// Deployment running details
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentRunningDetails {
	/// if the deployment should deploy as soon as a new image digest is pushed
	pub deploy_on_push: bool,
	/// The minimum number node a deployment should maintain
	pub min_horizontal_scale: u16,
	/// The maximum number of node deployment can scale up to at peak resource requirement
	pub max_horizontal_scale: u16,
	/// List of deployment port number of its type
	pub ports: BTreeMap<StringifiedU16, ExposedPortType>,
	/// List of environment variables are values
	pub environment_variables: BTreeMap<String, EnvironmentVariableValue>,
	/// The startup probe of a deployment if any
	#[serde(skip_serializing_if = "Option::is_none")]
	pub startup_probe: Option<DeploymentProbe>,
	/// The liveness probe of a deployment if any
	#[serde(skip_serializing_if = "Option::is_none")]
	pub liveness_probe: Option<DeploymentProbe>,
	/// The config map attached to a deployment
	pub config_mounts: BTreeMap<String, Base64String>,
	/// The volume attached to a deployment
	pub volumes: BTreeMap<String, DeploymentVolume>,
}

/// Deployment volume detail
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentVolume {
	/// The path of the volume attached
	pub path: String,
	/// The size of the volume
	pub size: u16,
}

/// The type of environment variable
/// The keys can either have a string as a value or a secret
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(tag = "type", rename_all = "camelCase")]
#[sqlx(type_name = "EXPOSED_PORT_TYPE", rename_all = "lowercase")]
pub enum ExposedPortType {
	Tcp,
	Udp,
	Http,
}

/// The type of exposed port 
#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct DeploymentProbe {
	/// The port the probe will be using
	pub port: u16,
	/// The path of the file to the probe commands
	pub path: String,
}

/// Patr registry
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct PatrRegistry;

struct PatrRegistryVisitor;

impl<'de> Deserialize<'de> for PatrRegistry {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_str(PatrRegistryVisitor)
	}
}

impl<'de> Visitor<'de> for PatrRegistryVisitor {
	type Value = PatrRegistry;

	fn expecting(
		&self,
		formatter: &mut std::fmt::Formatter,
	) -> std::fmt::Result {
		formatter.write_str(&format!(
			"a constant `{}` value",
			constants::CONTAINER_REGISTRY_URL
		))
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		if v == constants::CONTAINER_REGISTRY_URL {
			Ok(PatrRegistry)
		} else {
			Err(E::custom(format!(
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
		matches!(self, DeploymentRegistry::PatrRegistry { .. })
	}

	/// External registry
	pub fn is_external_registry(&self) -> bool {
		matches!(self, DeploymentRegistry::ExternalRegistry { .. })
	}
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "camelCase")]
#[sqlx(type_name = "DEPLOYMENT_STATUS", rename_all = "lowercase")]
pub enum DeploymentStatus {
	Created,
	Pushed,
	Deploying,
	Running,
	Stopped,
	Errored,
	Deleted,
}

/// All the possible deployment status a deployment can be 
/// in during its life cycle
#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeploymentStatus {
	/// Deployment has been created
	Created,
	/// Image of a deployment has been pushed
	Pushed,
	/// Deployment is deploying
	Deploying,
	/// Deployment is running
	Running,
	/// Deployment has stopped
	Stopped,
	/// Deployment has errored and stopped
	Errored,
	/// Deployment has been deleted
	Deleted,
}

impl Display for DeploymentStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Created => write!(f, "created"),
			Self::Pushed => write!(f, "pushed"),
			Self::Deploying => write!(f, "deploying"),
			Self::Running => write!(f, "running"),
			Self::Stopped => write!(f, "stopped"),
			Self::Errored => write!(f, "errored"),
			Self::Deleted => write!(f, "deleted"),
		}
	}
}

impl FromStr for DeploymentStatus {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.to_lowercase();
		match s.as_str() {
			"created" => Ok(Self::Created),
			"pushed" => Ok(Self::Pushed),
			"deploying" => Ok(Self::Deploying),
			"running" => Ok(Self::Running),
			"stopped" => Ok(Self::Stopped),
			"errored" => Ok(Self::Errored),
			"deleted" => Ok(Self::Deleted),
			_ => Err(s),
		}
	}
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "camelCase")]
#[sqlx(type_name = "DEPLOYMENT_STATUS", rename_all = "lowercase")]
pub enum StatefulSetStatus {
	Created,
	Pushed,
	Deploying,
	Running,
	Stopped,
	Errored,
	Deleted,
}

/// All the possible StatefulSet status a StatefulSet can be 
/// in during its life cycle
#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StatefulSetStatus {
	/// StatefulSet has been created
	Created,
	/// Image of a StatefulSet has been pushed
	Pushed,
	/// StatefulSet is deploying
	Deploying,
	/// StatefulSet is running
	Running,
	/// StatefulSet has stopped
	Stopped,
	/// StatefulSet has errored and stopped
	Errored,
	/// StatefulSet has been deleted
	Deleted,
}

impl Display for StatefulSetStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Created => write!(f, "created"),
			Self::Pushed => write!(f, "pushed"),
			Self::Deploying => write!(f, "deploying"),
			Self::Running => write!(f, "running"),
			Self::Stopped => write!(f, "stopped"),
			Self::Errored => write!(f, "errored"),
			Self::Deleted => write!(f, "deleted"),
		}
	}
}

impl FromStr for StatefulSetStatus {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.to_lowercase();
		match s.as_str() {
			"created" => Ok(Self::Created),
			"pushed" => Ok(Self::Pushed),
			"deploying" => Ok(Self::Deploying),
			"running" => Ok(Self::Running),
			"stopped" => Ok(Self::Stopped),
			"errored" => Ok(Self::Errored),
			"deleted" => Ok(Self::Deleted),
			_ => Err(s),
		}
	}
}

/// Deployment metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentMetrics {
	/// Pod name of the deployment
	pub pod_name: String,
	/// List of metrics of type Metric
	pub metrics: Vec<Metric>,
}

/// Metrics of a deployment pod
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Metric {
	/// The timestamp of the metric
	pub timestamp: u64,
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
pub struct DeploymentLogs {
	/// Timestamp of a deployment log
	pub timestamp: OffsetDateTime,
	/// The logs of a deployment
	pub logs: String,
}

/// Build logs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildLog {
	/// The timestamp of the build log
	pub timestamp: Option<u64>,
	/// The type of build log
	pub reason: Option<String>,
	/// The log
	pub message: Option<String>,
}

/// The time duration of a log
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Step {
	/// One minute
	OneMinute,
	/// Two minute
	TwoMinutes,
	/// Five minute
	FiveMinutes,
	/// Ten minute
	TenMinutes,
	/// Fifteen minute
	FifteenMinutes,
	/// Thirty minute
	ThirtyMinutes,
	/// One hour
	OneHour,
}

impl Display for Step {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::OneMinute => write!(f, "1m"),
			Self::TwoMinutes => write!(f, "2m"),
			Self::FiveMinutes => write!(f, "5m"),
			Self::TenMinutes => write!(f, "10m"),
			Self::FifteenMinutes => write!(f, "15m"),
			Self::ThirtyMinutes => write!(f, "30m"),
			Self::OneHour => write!(f, "1h"),
		}
	}
}

impl FromStr for Step {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.to_lowercase();
		match s.as_str() {
			"1m" => Ok(Self::OneMinute),
			"2m" => Ok(Self::TwoMinutes),
			"5m" => Ok(Self::FiveMinutes),
			"10m" => Ok(Self::TenMinutes),
			"15m" => Ok(Self::FifteenMinutes),
			"30m" => Ok(Self::ThirtyMinutes),
			"1h" => Ok(Self::OneHour),
			_ => Err(s),
		}
	}
}

/// Internval
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Interval {
	/// Hour
	Hour,
	/// Day
	Day,
	/// Week
	Week,
	/// Month
	Month,
	/// Year
	Year,
}

impl Display for Interval {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Hour => write!(f, "hour"),
			Self::Day => write!(f, "day"),
			Self::Week => write!(f, "week"),
			Self::Month => write!(f, "month"),
			Self::Year => write!(f, "year"),
		}
	}
}

impl Interval {
	/// Get internval as u64
	pub fn as_u64(&self) -> u64 {
		match self {
			Interval::Hour => todo!("Current time in seconds - 3600"),
			Interval::Day => todo!("Current time in seconds - 86400"),
			Interval::Week => todo!("Current time in seconds - 604800"),
			Interval::Month => todo!("Current time in seconds - 2628000"),
			Interval::Year => todo!("Current time in seconds - 31556952"),
		}
	}
}

impl FromStr for Interval {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let s = s.to_lowercase();
		match s.as_str() {
			"hour" | "hr" | "h" => Ok(Self::Hour),
			"day" | "d" => Ok(Self::Day),
			"week" | "w" => Ok(Self::Week),
			"month" | "mnth" | "m" => Ok(Self::Month),
			"year" | "yr" | "y" => Ok(Self::Year),
			_ => Err(s),
		}
	}
}

// #[cfg(test)]
// mod test {
// 	use std::collections::BTreeMap;

// 	use serde_test::{assert_tokens, Token};

// 	use super::{
// 		Deployment,
// 		DeploymentDeployHistory,
// 		DeploymentProbe,
// 		DeploymentRegistry,
// 		DeploymentRunningDetails,
// 		DeploymentStatus,
// 		DeploymentVolume,
// 		EnvironmentVariableValue,
// 		ExposedPortType,
// 		PatrRegistry,
// 	};
// 	use crate::utils::{constants, StringifiedU16, Uuid};

// 	#[test]
// 	fn assert_deployment_types_internal_registry() {
// 		assert_tokens(
// 			&Deployment {
// 				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
// 					.unwrap(),
// 				name: "John Patr's deployment".to_string(),
// 				registry: DeploymentRegistry::PatrRegistry {
// 					registry: PatrRegistry,
// 					repository_id: Uuid::parse_str(
// 						"2aef18631ded45eb9170dc2166b30867",
// 					)
// 					.unwrap(),
// 				},
// 				image_tag: "stable".to_string(),
// 				status: DeploymentStatus::Created,
// 				region: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
// 					.unwrap(),
// 				machine_type: Uuid::parse_str(
// 					"2aef18631ded45eb9170dc2166b30867",
// 				)
// 				.unwrap(),
// 				current_live_digest: Some(
// 					"sha256:2aef18631ded45eb9170dc2166b30867".to_string(),
// 				),
// 			},
// 			&[
// 				Token::Map { len: None },
// 				Token::Str("id"),
// 				Token::Str("2aef18631ded45eb9170dc2166b30867"),
// 				Token::Str("name"),
// 				Token::Str("John Patr's deployment"),
// 				Token::Str("registry"),
// 				Token::Str(constants::CONTAINER_REGISTRY_URL),
// 				Token::Str("repositoryId"),
// 				Token::Str("2aef18631ded45eb9170dc2166b30867"),
// 				Token::Str("imageTag"),
// 				Token::Str("stable"),
// 				Token::Str("status"),
// 				Token::UnitVariant {
// 					name: "DeploymentStatus",
// 					variant: "created",
// 				},
// 				Token::Str("region"),
// 				Token::Str("2aef18631ded45eb9170dc2166b30867"),
// 				Token::Str("machineType"),
// 				Token::Str("2aef18631ded45eb9170dc2166b30867"),
// 				Token::Str("currentLiveDigest"),
// 				Token::Some,
// 				Token::Str("sha256:2aef18631ded45eb9170dc2166b30867"),
// 				Token::MapEnd,
// 			],
// 		)
// 	}

// 	#[test]
// 	fn assert_deployment_types_external_registry() {
// 		assert_tokens(
// 			&Deployment {
// 				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
// 					.unwrap(),
// 				name: "John Patr's deployment".to_string(),
// 				registry: DeploymentRegistry::ExternalRegistry {
// 					registry: "registry.hub.docker.com".to_string(),
// 					image_name: "johnpatr/deployment".to_string(),
// 				},
// 				image_tag: "stable".to_string(),
// 				status: DeploymentStatus::Created,
// 				region: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
// 					.unwrap(),
// 				machine_type: Uuid::parse_str(
// 					"2aef18631ded45eb9170dc2166b30867",
// 				)
// 				.unwrap(),
// 				current_live_digest: Some(
// 					"sha256:2aef18631ded45eb9170dc2166b30867".to_string(),
// 				),
// 			},
// 			&[
// 				Token::Map { len: None },
// 				Token::Str("id"),
// 				Token::Str("2aef18631ded45eb9170dc2166b30867"),
// 				Token::Str("name"),
// 				Token::Str("John Patr's deployment"),
// 				Token::Str("registry"),
// 				Token::Str("registry.hub.docker.com"),
// 				Token::Str("imageName"),
// 				Token::Str("johnpatr/deployment"),
// 				Token::Str("imageTag"),
// 				Token::Str("stable"),
// 				Token::Str("status"),
// 				Token::UnitVariant {
// 					name: "DeploymentStatus",
// 					variant: "created",
// 				},
// 				Token::Str("region"),
// 				Token::Str("2aef18631ded45eb9170dc2166b30867"),
// 				Token::Str("machineType"),
// 				Token::Str("2aef18631ded45eb9170dc2166b30867"),
// 				Token::Str("currentLiveDigest"),
// 				Token::Some,
// 				Token::Str("sha256:2aef18631ded45eb9170dc2166b30867"),
// 				Token::MapEnd,
// 			],
// 		)
// 	}

// 	#[test]
// 	fn assert_deployment_deploy_history() {
// 		assert_tokens(
// 			&DeploymentDeployHistory {
// 				image_digest: "sha256:2aef18631ded45eb9170dc2166b30867"
// 					.to_string(),
// 				created: 6789123712,
// 			},
// 			&[
// 				Token::Struct {
// 					name: "DeploymentDeployHistory",
// 					len: 2,
// 				},
// 				Token::Str("imageDigest"),
// 				Token::Str("sha256:2aef18631ded45eb9170dc2166b30867"),
// 				Token::Str("created"),
// 				Token::U64(6789123712),
// 				Token::StructEnd,
// 			],
// 		)
// 	}

// 	#[test]
// 	fn assert_deployment_running_details_types() {
// 		assert_tokens(
// 			&DeploymentRunningDetails {
// 				deploy_on_push: true,
// 				min_horizontal_scale: 1,
// 				max_horizontal_scale: 2,
// 				ports: {
// 					let mut map = BTreeMap::new();

// 					map.insert(
// 						StringifiedU16::new(3000),
// 						ExposedPortType::Http,
// 					);
// 					map.insert(StringifiedU16::new(8080), ExposedPortType::Tcp);

// 					map
// 				},
// 				environment_variables: {
// 					let mut map = BTreeMap::new();

// 					map.insert(
// 						"APP_PORT".to_string(),
// 						EnvironmentVariableValue::String("3000".to_string()),
// 					);
// 					map.insert(
// 						"APP_JWT_PASSWORD".to_string(),
// 						EnvironmentVariableValue::Secret {
// 							from_secret: Uuid::parse_str(
// 								"2aef18631ded45eb9170dc2166b30867",
// 							)
// 							.unwrap(),
// 						},
// 					);

// 					map
// 				},
// 				startup_probe: Some(DeploymentProbe {
// 					port: 8080,
// 					path: "/health".to_string(),
// 				}),
// 				liveness_probe: Some(DeploymentProbe {
// 					port: 8080,
// 					path: "/health".to_string(),
// 				}),
// 				config_mounts: {
// 					let mut map = BTreeMap::new();

// 					map.insert(
// 						"/app/config.json".to_string(),
// 						b"fdbuasgdsgaosueaghwehhgw8hguwegheoghe"
// 							.to_vec()
// 							.into(),
// 					);

// 					map
// 				},
// 				volumes: {
// 					let mut map = BTreeMap::new();
// 					map.insert(
// 						"v1".to_string(),
// 						DeploymentVolume {
// 							path: "/volume".to_string(),
// 							size: 10,
// 						},
// 					);
// 					map
// 				},
// 			},
// 			&[
// 				Token::Struct {
// 					name: "DeploymentRunningDetails",
// 					len: 9,
// 				},
// 				Token::Str("deployOnPush"),
// 				Token::Bool(true),
// 				Token::Str("minHorizontalScale"),
// 				Token::U16(1),
// 				Token::Str("maxHorizontalScale"),
// 				Token::U16(2),
// 				Token::Str("ports"),
// 				Token::Map { len: Some(2) },
// 				Token::Str("3000"),
// 				Token::Struct {
// 					name: "ExposedPortType",
// 					len: 1,
// 				},
// 				Token::Str("type"),
// 				Token::Str("http"),
// 				Token::StructEnd,
// 				Token::Str("8080"),
// 				Token::Struct {
// 					name: "ExposedPortType",
// 					len: 1,
// 				},
// 				Token::Str("type"),
// 				Token::Str("tcp"),
// 				Token::StructEnd,
// 				Token::MapEnd,
// 				Token::Str("environmentVariables"),
// 				Token::Map { len: Some(2) },
// 				Token::Str("APP_JWT_PASSWORD"),
// 				Token::Struct {
// 					name: "EnvironmentVariableValue",
// 					len: 1,
// 				},
// 				Token::Str("fromSecret"),
// 				Token::Str("2aef18631ded45eb9170dc2166b30867"),
// 				Token::StructEnd,
// 				Token::Str("APP_PORT"),
// 				Token::Str("3000"),
// 				Token::MapEnd,
// 				Token::Str("startupProbe"),
// 				Token::Some,
// 				Token::Struct {
// 					name: "DeploymentProbe",
// 					len: 2,
// 				},
// 				Token::Str("port"),
// 				Token::U16(8080),
// 				Token::Str("path"),
// 				Token::Str("/health"),
// 				Token::StructEnd,
// 				Token::Str("livenessProbe"),
// 				Token::Some,
// 				Token::Struct {
// 					name: "DeploymentProbe",
// 					len: 2,
// 				},
// 				Token::Str("port"),
// 				Token::U16(8080),
// 				Token::Str("path"),
// 				Token::Str("/health"),
// 				Token::StructEnd,
// 				Token::Str("configMounts"),
// 				Token::Map { len: Some(1) },
// 				Token::Str("/app/config.json"),
// 				Token::Str(
// 					"ZmRidWFzZ2RzZ2Fvc3VlYWdod2VoaGd3OGhndXdlZ2hlb2doZQ==",
// 				),
// 				Token::MapEnd,
// 				Token::Str("volumes"),
// 				Token::Map { len: Some(1) },
// 				Token::Str("v1"),
// 				Token::Struct {
// 					name: "DeploymentVolume",
// 					len: 2,
// 				},
// 				Token::Str("path"),
// 				Token::Str("/volume"),
// 				Token::Str("size"),
// 				Token::U16(10),
// 				Token::StructEnd,
// 				Token::MapEnd,
// 				Token::StructEnd,
// 			],
// 		)
// 	}

// 	#[test]
// 	fn assert_all_deployment_status_types() {
// 		for status in [
// 			DeploymentStatus::Created,
// 			DeploymentStatus::Pushed,
// 			DeploymentStatus::Deploying,
// 			DeploymentStatus::Running,
// 			DeploymentStatus::Stopped,
// 			DeploymentStatus::Errored,
// 			DeploymentStatus::Deleted,
// 		] {
// 			assert_tokens(
// 				&status,
// 				&[Token::UnitVariant {
// 					name: "DeploymentStatus",
// 					variant: match &status {
// 						DeploymentStatus::Created => "created",
// 						DeploymentStatus::Pushed => "pushed",
// 						DeploymentStatus::Deploying => "deploying",
// 						DeploymentStatus::Running => "running",
// 						DeploymentStatus::Stopped => "stopped",
// 						DeploymentStatus::Errored => "errored",
// 						DeploymentStatus::Deleted => "deleted",
// 					},
// 				}],
// 			);
// 		}
// 	}

// 	#[test]
// 	fn assert_internal_deployment_registry_types() {
// 		assert_tokens(
// 			&DeploymentRegistry::PatrRegistry {
// 				registry: PatrRegistry,
// 				repository_id: Uuid::parse_str(
// 					"2aef18631ded45eb9170dc2166b30867",
// 				)
// 				.unwrap(),
// 			},
// 			&[
// 				Token::Struct {
// 					name: "DeploymentRegistry",
// 					len: 2,
// 				},
// 				Token::Str("registry"),
// 				Token::Str(constants::CONTAINER_REGISTRY_URL),
// 				Token::Str("repositoryId"),
// 				Token::Str("2aef18631ded45eb9170dc2166b30867"),
// 				Token::StructEnd,
// 			],
// 		)
// 	}

// 	#[test]
// 	fn assert_external_deployment_registry_types() {
// 		assert_tokens(
// 			&DeploymentRegistry::ExternalRegistry {
// 				registry: "registry.hub.docker.com".to_string(),
// 				image_name: "johnpatr/deployment".to_string(),
// 			},
// 			&[
// 				Token::Struct {
// 					name: "DeploymentRegistry",
// 					len: 2,
// 				},
// 				Token::Str("registry"),
// 				Token::Str("registry.hub.docker.com"),
// 				Token::Str("imageName"),
// 				Token::Str("johnpatr/deployment"),
// 				Token::StructEnd,
// 			],
// 		)
// 	}

// 	#[test]
// 	fn assert_patr_registry_types() {
// 		assert_tokens(&PatrRegistry, &[Token::Str(constants::CONTAINER_REGISTRY_URL)])
// 	}

// 	#[test]
// 	fn assert_exposed_port_type_types() {
// 		for exposed_port_type in [
// 			ExposedPortType::Http,
// 			ExposedPortType::Tcp,
// 			ExposedPortType::Udp,
// 		] {
// 			assert_tokens(
// 				&exposed_port_type,
// 				&[
// 					Token::Struct {
// 						name: "ExposedPortType",
// 						len: 1,
// 					},
// 					Token::Str("type"),
// 					Token::Str(match &exposed_port_type {
// 						ExposedPortType::Http => "http",
// 						ExposedPortType::Tcp => "tcp",
// 						ExposedPortType::Udp => "udp",
// 					}),
// 					Token::StructEnd,
// 				],
// 			)
// 		}
// 	}

// 	#[test]
// 	fn assert_environment_variable_value_string_types() {
// 		assert_tokens(
// 			&EnvironmentVariableValue::String("test".to_string()),
// 			&[Token::Str("test")],
// 		)
// 	}

// 	#[test]
// 	fn assert_environment_variable_value_secret_types() {
// 		assert_tokens(
// 			&EnvironmentVariableValue::Secret {
// 				from_secret: Uuid::parse_str(
// 					"2aef18631ded45eb9170dc2166b30867",
// 				)
// 				.unwrap(),
// 			},
// 			&[
// 				Token::Struct {
// 					name: "EnvironmentVariableValue",
// 					len: 1,
// 				},
// 				Token::Str("fromSecret"),
// 				Token::Str("2aef18631ded45eb9170dc2166b30867"),
// 				Token::StructEnd,
// 			],
// 		)
// 	}
// }
