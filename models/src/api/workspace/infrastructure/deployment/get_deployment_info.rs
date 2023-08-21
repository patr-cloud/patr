use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::{Deployment, DeploymentRunningDetails};
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
#[typed_path(
	"/workspace/:workspace_id/infrastructure/deployment/:deployment_id/"
)]
pub struct GetDeploymentInfoPath {
	pub workspace_id: Uuid,
	pub deployment_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDeploymentInfoRequest {}

impl ApiRequest for GetDeploymentInfoRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetDeploymentInfoPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetDeploymentInfoResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetDeploymentInfoResponse {
	#[serde(flatten)]
	pub deployment: Deployment,
	#[serde(flatten)]
	pub running_details: DeploymentRunningDetails,
}

#[cfg(test)]
mod test {
	use std::collections::BTreeMap;

	use serde_test::{assert_tokens, Token};

	use super::{GetDeploymentInfoRequest, GetDeploymentInfoResponse};
	use crate::{
		models::workspace::infrastructure::deployment::{
			Deployment,
			DeploymentProbe,
			DeploymentRegistry,
			DeploymentRunningDetails,
			DeploymentStatus,
			DeploymentVolume,
			EnvironmentVariableValue,
			ExposedPortType,
			PatrRegistry,
		},
		utils::{constants, StringifiedU16, Uuid},
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetDeploymentInfoRequest {},
			&[
				Token::Struct {
					name: "GetDeploymentInfoRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetDeploymentInfoResponse {
				deployment: Deployment {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "John Patr's deployment".to_string(),
					registry: DeploymentRegistry::PatrRegistry {
						registry: PatrRegistry,
						repository_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
					},
					image_tag: "latest".to_string(),
					status: DeploymentStatus::Running,
					region: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					machine_type: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30867",
					)
					.unwrap(),
					current_live_digest: Some(
						"sha256:2aef18631ded45eb9170dc2166b30867".to_string(),
					),
				},
				running_details: DeploymentRunningDetails {
					deploy_on_push: true,
					min_horizontal_scale: 1,
					max_horizontal_scale: 2,
					ports: {
						let mut map = BTreeMap::new();

						map.insert(
							StringifiedU16::new(3000),
							ExposedPortType::Http,
						);
						map.insert(
							StringifiedU16::new(8080),
							ExposedPortType::Tcp,
						);

						map
					},
					environment_variables: {
						let mut map = BTreeMap::new();

						map.insert(
							"APP_PORT".to_string(),
							EnvironmentVariableValue::String(
								"3000".to_string(),
							),
						);
						map.insert(
							"APP_JWT_PASSWORD".to_string(),
							EnvironmentVariableValue::Secret {
								from_secret: Uuid::parse_str(
									"2aef18631ded45eb9170dc2166b30867",
								)
								.unwrap(),
							},
						);

						map
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
			},
			&[
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("John Patr's deployment"),
				Token::Str("registry"),
				Token::Str(constants::PATR_REGISTRY),
				Token::Str("repositoryId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("imageTag"),
				Token::Str("latest"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "DeploymentStatus",
					variant: "running",
				},
				Token::Str("region"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("machineType"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("currentLiveDigest"),
				Token::Some,
				Token::Str("sha256:2aef18631ded45eb9170dc2166b30867"),
				Token::Str("deployOnPush"),
				Token::Bool(true),
				Token::Str("minHorizontalScale"),
				Token::U16(1),
				Token::Str("maxHorizontalScale"),
				Token::U16(2),
				Token::Str("ports"),
				Token::Map { len: Some(2) },
				Token::Str("3000"),
				Token::Struct {
					name: "ExposedPortType",
					len: 1,
				},
				Token::Str("type"),
				Token::Str("http"),
				Token::StructEnd,
				Token::Str("8080"),
				Token::Struct {
					name: "ExposedPortType",
					len: 1,
				},
				Token::Str("type"),
				Token::Str("tcp"),
				Token::StructEnd,
				Token::MapEnd,
				Token::Str("environmentVariables"),
				Token::Map { len: Some(2) },
				Token::Str("APP_JWT_PASSWORD"),
				Token::Struct {
					name: "EnvironmentVariableValue",
					len: 1,
				},
				Token::Str("fromSecret"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
				Token::Str("APP_PORT"),
				Token::Str("3000"),
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
				Token::MapEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetDeploymentInfoResponse {
				deployment: Deployment {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "John Patr's deployment".to_string(),
					registry: DeploymentRegistry::PatrRegistry {
						registry: PatrRegistry,
						repository_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
					},
					image_tag: "latest".to_string(),
					status: DeploymentStatus::Running,
					region: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					machine_type: Uuid::parse_str(
						"2aef18631ded45eb9170dc2166b30867",
					)
					.unwrap(),
					current_live_digest: Some(
						"sha256:2aef18631ded45eb9170dc2166b30867".to_string(),
					),
				},
				running_details: DeploymentRunningDetails {
					deploy_on_push: true,
					min_horizontal_scale: 1,
					max_horizontal_scale: 2,
					ports: {
						let mut map = BTreeMap::new();

						map.insert(
							StringifiedU16::new(3000),
							ExposedPortType::Http,
						);
						map.insert(
							StringifiedU16::new(8080),
							ExposedPortType::Tcp,
						);

						map
					},
					environment_variables: {
						let mut map = BTreeMap::new();

						map.insert(
							"APP_PORT".to_string(),
							EnvironmentVariableValue::String(
								"3000".to_string(),
							),
						);
						map.insert(
							"APP_JWT_PASSWORD".to_string(),
							EnvironmentVariableValue::Secret {
								from_secret: Uuid::parse_str(
									"2aef18631ded45eb9170dc2166b30867",
								)
								.unwrap(),
							},
						);

						map
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
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("John Patr's deployment"),
				Token::Str("registry"),
				Token::Str(constants::PATR_REGISTRY),
				Token::Str("repositoryId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("imageTag"),
				Token::Str("latest"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "DeploymentStatus",
					variant: "running",
				},
				Token::Str("region"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("machineType"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("currentLiveDigest"),
				Token::Some,
				Token::Str("sha256:2aef18631ded45eb9170dc2166b30867"),
				Token::Str("deployOnPush"),
				Token::Bool(true),
				Token::Str("minHorizontalScale"),
				Token::U16(1),
				Token::Str("maxHorizontalScale"),
				Token::U16(2),
				Token::Str("ports"),
				Token::Map { len: Some(2) },
				Token::Str("3000"),
				Token::Struct {
					name: "ExposedPortType",
					len: 1,
				},
				Token::Str("type"),
				Token::Str("http"),
				Token::StructEnd,
				Token::Str("8080"),
				Token::Struct {
					name: "ExposedPortType",
					len: 1,
				},
				Token::Str("type"),
				Token::Str("tcp"),
				Token::StructEnd,
				Token::MapEnd,
				Token::Str("environmentVariables"),
				Token::Map { len: Some(2) },
				Token::Str("APP_JWT_PASSWORD"),
				Token::Struct {
					name: "EnvironmentVariableValue",
					len: 1,
				},
				Token::Str("fromSecret"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
				Token::Str("APP_PORT"),
				Token::Str("3000"),
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
				Token::MapEnd,
			],
		)
	}
}
