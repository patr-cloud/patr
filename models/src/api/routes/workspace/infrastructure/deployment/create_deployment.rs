use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::{DeploymentRegistry, DeploymentRunningDetails};
use crate::{utils::Uuid, ApiRequest};

#[derive(
	Eq,
	Ord,
	Hash,
	Debug,
	Clone,
	Default,
	TypedPath,
	PartialEq,
	Serialize,
	PartialOrd,
	Deserialize,
)]
#[typed_path("/workspace/:workspace_id/infrastructure/deployment")]
pub struct CreateDeploymentPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeploymentRequest {
	pub name: String,
	#[serde(flatten)]
	pub registry: DeploymentRegistry,
	pub image_tag: String,
	pub region: Uuid,
	pub machine_type: Uuid,
	#[serde(flatten)]
	pub running_details: DeploymentRunningDetails,
	pub deploy_on_create: bool,
}

impl ApiRequest for CreateDeploymentRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = CreateDeploymentPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = CreateDeploymentResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateDeploymentResponse {
	pub id: Uuid,
}

#[cfg(test)]
mod test {
	use std::collections::BTreeMap;

	use serde_test::{assert_tokens, Token};

	use super::{CreateDeploymentRequest, CreateDeploymentResponse};
	use crate::{
		models::workspace::infrastructure::deployment::{
			DeploymentProbe,
			DeploymentRegistry,
			DeploymentRunningDetails,
			DeploymentVolume,
			EnvironmentVariableValue,
			ExposedPortType,
			PatrRegistry,
		},
		utils::{constants, StringifiedU16, Uuid},
		ApiResponse,
	};

	#[test]
	fn assert_request_with_internal_registry_types() {
		assert_tokens(
			&CreateDeploymentRequest {
				name: "John Patr's deployment".to_string(),
				registry: DeploymentRegistry::PatrRegistry {
					registry: PatrRegistry,
					repository_id: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30867",
					)
					.unwrap(),
				},
				image_tag: "stable".to_string(),
				region: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				machine_type: Uuid::parse_str(
					"2aef18631ded45eb9170dc2166b30867",
				)
				.unwrap(),
				running_details: DeploymentRunningDetails {
					deploy_on_push: true,
					min_horizontal_scale: 1,
					max_horizontal_scale: 2,
					ports: {
						let mut ports = BTreeMap::new();

						ports.insert(
							StringifiedU16::new(8080),
							ExposedPortType::Http,
						);
						ports.insert(
							StringifiedU16::new(3306),
							ExposedPortType::Tcp,
						);

						ports
					},
					environment_variables: {
						let mut env_vars =
							BTreeMap::<String, EnvironmentVariableValue>::new();

						env_vars.insert(
							"DB_HOST".to_string(),
							EnvironmentVariableValue::String(
								"localhost".to_string(),
							),
						);
						env_vars.insert(
							"DB_PORT".to_string(),
							EnvironmentVariableValue::Secret {
								from_secret: Uuid::parse_str(
									"2aef18631ded45eb9170dc2166b30867",
								)
								.unwrap(),
							},
						);

						env_vars
					},
					startup_probe: Some(DeploymentProbe {
						port: 8080,
						path: "/health".to_string(),
					}),
					liveness_probe: Some(DeploymentProbe {
						port: 8080,
						path: "/health".to_string(),
					}),
					config_mounts: {
						let mut map = BTreeMap::new();

						map.insert(
							"/app/config.json".to_string(),
							b"fdbuasgdsgaosueaghwehhgw8hguwegheoghe"
								.to_vec()
								.into(),
						);

						map
					},
					volumes: {
						let mut map = BTreeMap::new();
						map.insert(
							"v1".to_string(),
							DeploymentVolume {
								path: "/volume".to_string(),
								size: 10,
							},
						);
						map
					},
				},
				deploy_on_create: true,
			},
			&[
				Token::Map { len: None },
				Token::Str("name"),
				Token::Str("John Patr's deployment"),
				Token::Str("registry"),
				Token::Str(constants::PATR_REGISTRY),
				Token::Str("repositoryId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("imageTag"),
				Token::Str("stable"),
				Token::Str("region"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("machineType"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("deployOnPush"),
				Token::Bool(true),
				Token::Str("minHorizontalScale"),
				Token::U16(1),
				Token::Str("maxHorizontalScale"),
				Token::U16(2),
				Token::Str("ports"),
				Token::Map { len: Some(2) },
				Token::Str("3306"),
				Token::Struct {
					name: "ExposedPortType",
					len: 1,
				},
				Token::Str("type"),
				Token::Str("tcp"),
				Token::StructEnd,
				Token::Str("8080"),
				Token::Struct {
					name: "ExposedPortType",
					len: 1,
				},
				Token::Str("type"),
				Token::Str("http"),
				Token::StructEnd,
				Token::MapEnd,
				Token::Str("environmentVariables"),
				Token::Map { len: Some(2) },
				Token::Str("DB_HOST"),
				Token::Str("localhost"),
				Token::Str("DB_PORT"),
				Token::Struct {
					name: "EnvironmentVariableValue",
					len: 1,
				},
				Token::Str("fromSecret"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
				Token::MapEnd,
				Token::Str("startupProbe"),
				Token::Some,
				Token::Struct {
					name: "DeploymentProbe",
					len: 2,
				},
				Token::Str("port"),
				Token::U16(8080),
				Token::Str("path"),
				Token::Str("/health"),
				Token::StructEnd,
				Token::Str("livenessProbe"),
				Token::Some,
				Token::Struct {
					name: "DeploymentProbe",
					len: 2,
				},
				Token::Str("port"),
				Token::U16(8080),
				Token::Str("path"),
				Token::Str("/health"),
				Token::StructEnd,
				Token::Str("configMounts"),
				Token::Map { len: Some(1) },
				Token::Str("/app/config.json"),
				Token::Str(
					"ZmRidWFzZ2RzZ2Fvc3VlYWdod2VoaGd3OGhndXdlZ2hlb2doZQ==",
				),
				Token::MapEnd,
				Token::Str("volumes"),
				Token::Map { len: Some(1) },
				Token::Str("v1"),
				Token::Struct {
					name: "DeploymentVolume",
					len: 2,
				},
				Token::Str("path"),
				Token::Str("/volume"),
				Token::Str("size"),
				Token::U16(10),
				Token::StructEnd,
				Token::MapEnd,
				Token::Str("deployOnCreate"),
				Token::Bool(true),
				Token::MapEnd,
			],
		)
	}

	#[test]
	fn assert_request_with_external_registry_types() {
		assert_tokens(
			&CreateDeploymentRequest {
				name: "John Patr's deployment".to_string(),
				registry: DeploymentRegistry::ExternalRegistry {
					registry: "registry.hub.docker.com".to_string(),
					image_name: "johnpatr/deployment".to_string(),
				},
				image_tag: "stable".to_string(),
				region: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				machine_type: Uuid::parse_str(
					"2aef18631ded45eb9170dc2166b30867",
				)
				.unwrap(),
				running_details: DeploymentRunningDetails {
					deploy_on_push: true,
					min_horizontal_scale: 1,
					max_horizontal_scale: 2,
					ports: {
						let mut ports = BTreeMap::new();

						ports.insert(
							StringifiedU16::new(8080),
							ExposedPortType::Http,
						);
						ports.insert(
							StringifiedU16::new(3306),
							ExposedPortType::Tcp,
						);

						ports
					},
					environment_variables: {
						let mut env_vars =
							BTreeMap::<String, EnvironmentVariableValue>::new();

						env_vars.insert(
							"DB_HOST".to_string(),
							EnvironmentVariableValue::String(
								"localhost".to_string(),
							),
						);
						env_vars.insert(
							"DB_PORT".to_string(),
							EnvironmentVariableValue::Secret {
								from_secret: Uuid::parse_str(
									"2aef18631ded45eb9170dc2166b30867",
								)
								.unwrap(),
							},
						);

						env_vars
					},
					startup_probe: Some(DeploymentProbe {
						port: 8080,
						path: "/health".to_string(),
					}),
					liveness_probe: Some(DeploymentProbe {
						port: 8080,
						path: "/health".to_string(),
					}),
					config_mounts: {
						let mut map = BTreeMap::new();

						map.insert(
							"/app/config.json".to_string(),
							b"fdbuasgdsgaosueaghwehhgw8hguwegheoghe"
								.to_vec()
								.into(),
						);

						map
					},
					volumes: {
						let mut map = BTreeMap::new();
						map.insert(
							"v1".to_string(),
							DeploymentVolume {
								path: "/volume".to_string(),
								size: 10,
							},
						);
						map
					},
				},
				deploy_on_create: true,
			},
			&[
				Token::Map { len: None },
				Token::Str("name"),
				Token::Str("John Patr's deployment"),
				Token::Str("registry"),
				Token::Str("registry.hub.docker.com"),
				Token::Str("imageName"),
				Token::Str("johnpatr/deployment"),
				Token::Str("imageTag"),
				Token::Str("stable"),
				Token::Str("region"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("machineType"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("deployOnPush"),
				Token::Bool(true),
				Token::Str("minHorizontalScale"),
				Token::U16(1),
				Token::Str("maxHorizontalScale"),
				Token::U16(2),
				Token::Str("ports"),
				Token::Map { len: Some(2) },
				Token::Str("3306"),
				Token::Struct {
					name: "ExposedPortType",
					len: 1,
				},
				Token::Str("type"),
				Token::Str("tcp"),
				Token::StructEnd,
				Token::Str("8080"),
				Token::Struct {
					name: "ExposedPortType",
					len: 1,
				},
				Token::Str("type"),
				Token::Str("http"),
				Token::StructEnd,
				Token::MapEnd,
				Token::Str("environmentVariables"),
				Token::Map { len: Some(2) },
				Token::Str("DB_HOST"),
				Token::Str("localhost"),
				Token::Str("DB_PORT"),
				Token::Struct {
					name: "EnvironmentVariableValue",
					len: 1,
				},
				Token::Str("fromSecret"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
				Token::MapEnd,
				Token::Str("startupProbe"),
				Token::Some,
				Token::Struct {
					name: "DeploymentProbe",
					len: 2,
				},
				Token::Str("port"),
				Token::U16(8080),
				Token::Str("path"),
				Token::Str("/health"),
				Token::StructEnd,
				Token::Str("livenessProbe"),
				Token::Some,
				Token::Struct {
					name: "DeploymentProbe",
					len: 2,
				},
				Token::Str("port"),
				Token::U16(8080),
				Token::Str("path"),
				Token::Str("/health"),
				Token::StructEnd,
				Token::Str("configMounts"),
				Token::Map { len: Some(1) },
				Token::Str("/app/config.json"),
				Token::Str(
					"ZmRidWFzZ2RzZ2Fvc3VlYWdod2VoaGd3OGhndXdlZ2hlb2doZQ==",
				),
				Token::MapEnd,
				Token::Str("volumes"),
				Token::Map { len: Some(1) },
				Token::Str("v1"),
				Token::Struct {
					name: "DeploymentVolume",
					len: 2,
				},
				Token::Str("path"),
				Token::Str("/volume"),
				Token::Str("size"),
				Token::U16(10),
				Token::StructEnd,
				Token::MapEnd,
				Token::Str("deployOnCreate"),
				Token::Bool(true),
				Token::MapEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&CreateDeploymentResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "CreateDeploymentResponse",
					len: 1,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(CreateDeploymentResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::MapEnd,
			],
		)
	}
}
