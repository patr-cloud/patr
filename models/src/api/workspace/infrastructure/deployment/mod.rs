use std::{collections::BTreeMap, fmt::Display, str::FromStr, time::SystemTime};

use chrono::Utc;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

mod create_deployment;
mod delete_deployment;
mod get_deployment_build_logs;
mod get_deployment_events;
mod get_deployment_info;
mod get_deployment_logs;
mod get_deployment_metrics;
mod list_deployment_history;
mod list_deployments;
mod list_linked_urls;
mod revert_deployment;
mod start_deployment;
mod stop_deployment;
mod update_deployment;

pub use self::{
	create_deployment::*,
	delete_deployment::*,
	get_deployment_build_logs::*,
	get_deployment_events::*,
	get_deployment_info::*,
	get_deployment_logs::*,
	get_deployment_metrics::*,
	list_deployment_history::*,
	list_deployments::*,
	list_linked_urls::*,
	revert_deployment::*,
	start_deployment::*,
	stop_deployment::*,
	update_deployment::*,
};
use crate::{prelude::*, utils::DateTime};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Deployment {
	pub name: String,
	#[serde(flatten)]
	pub registry: DeploymentRegistry,
	pub image_tag: String,
	pub status: DeploymentStatus,
	pub region: Uuid,
	pub machine_type: Uuid,
	pub current_live_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentDeployHistory {
	pub image_digest: String,
	pub created: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentRunningDetails {
	pub deploy_on_push: bool,
	pub min_horizontal_scale: u16,
	pub max_horizontal_scale: u16,
	pub ports: BTreeMap<StringifiedU16, ExposedPortType>,
	pub environment_variables: BTreeMap<String, EnvironmentVariableValue>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub startup_probe: Option<DeploymentProbe>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub liveness_probe: Option<DeploymentProbe>,
	pub config_mounts: BTreeMap<String, Base64String>,
	pub volumes: BTreeMap<String, DeploymentVolume>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentVolume {
	pub path: String,
	pub size: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum EnvironmentVariableValue {
	String(String),
	#[serde(rename_all = "camelCase")]
	Secret {
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

#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ExposedPortType {
	Tcp,
	Udp,
	Http,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentProbe {
	pub port: u16,
	pub path: String,
}
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum DeploymentRegistry {
	#[serde(rename_all = "camelCase")]
	PatrRegistry {
		registry: PatrRegistry,
		repository_id: Uuid,
	},
	#[serde(rename_all = "camelCase")]
	ExternalRegistry {
		registry: String,
		image_name: String,
	},
}

impl DeploymentRegistry {
	pub fn is_patr_registry(&self) -> bool {
		matches!(self, DeploymentRegistry::PatrRegistry { .. })
	}

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

#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeploymentStatus {
	Created,
	Pushed,
	Deploying,
	Running,
	Stopped,
	Errored,
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

#[cfg(not(feature = "server"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StatefulSetStatus {
	Created,
	Pushed,
	Deploying,
	Running,
	Stopped,
	Errored,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentMetrics {
	pub pod_name: String,
	pub metrics: Vec<Metric>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Metric {
	pub timestamp: u64,
	pub cpu_usage: String,
	pub memory_usage: String,
	pub network_usage_tx: String,
	pub network_usage_rx: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentLogs {
	pub timestamp: DateTime<Utc>,
	pub logs: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildLog {
	pub timestamp: Option<u64>,
	pub reason: Option<String>,
	pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Step {
	OneMinute,
	TwoMinutes,
	FiveMinutes,
	TenMinutes,
	FifteenMinutes,
	ThirtyMinutes,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Interval {
	Hour,
	Day,
	Week,
	Month,
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
	pub fn as_u64(&self) -> u64 {
		match self {
			Interval::Hour => get_current_time().as_secs() - 3600,
			Interval::Day => get_current_time().as_secs() - 86400,
			Interval::Week => get_current_time().as_secs() - 604800,
			Interval::Month => get_current_time().as_secs() - 2628000,
			Interval::Year => get_current_time().as_secs() - 31556952,
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
