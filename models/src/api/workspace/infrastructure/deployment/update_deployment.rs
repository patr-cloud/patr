use std::collections::BTreeMap;

use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::{
	DeploymentProbe,
	DeploymentVolume,
	EnvironmentVariableValue,
	ExposedPortType,
};
use crate::{
	utils::{Base64String, StringifiedU16, Uuid},
	ApiRequest,
};

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
pub struct UpdateDeploymentPath {
	pub workspace_id: Uuid,
	pub deployment_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDeploymentRequest {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub machine_type: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub deploy_on_push: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub min_horizontal_scale: Option<u16>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_horizontal_scale: Option<u16>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub ports: Option<BTreeMap<StringifiedU16, ExposedPortType>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub environment_variables:
		Option<BTreeMap<String, EnvironmentVariableValue>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub startup_probe: Option<DeploymentProbe>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub liveness_probe: Option<DeploymentProbe>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub config_mounts: Option<BTreeMap<String, Base64String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub volumes: Option<BTreeMap<String, DeploymentVolume>>,
}

impl ApiRequest for UpdateDeploymentRequest {
	const METHOD: Method = Method::PATCH;
	const IS_PROTECTED: bool = true;

	type RequestPath = UpdateDeploymentPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use std::collections::BTreeMap;

	use serde_test::{assert_tokens, Token};

	use super::UpdateDeploymentRequest;
	use crate::{
		models::workspace::infrastructure::deployment::{
			DeploymentProbe,
			DeploymentVolume,
			EnvironmentVariableValue,
			ExposedPortType,
		},
		utils::{StringifiedU16, Uuid},
		ApiRequest,
		ApiResponse,
	};

	#[test]
	fn assert_empty_request_types() {
		assert_tokens(
			&UpdateDeploymentRequest {
				name: None,
				machine_type: None,
				deploy_on_push: None,
				min_horizontal_scale: None,
				max_horizontal_scale: None,
				ports: None,
				environment_variables: None,
				startup_probe: None,
				liveness_probe: None,
				config_mounts: None,
				volumes: None,
			},
			&[
				Token::Struct {
					name: "UpdateDeploymentRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_filled_request_types() {
		assert_tokens(
			&UpdateDeploymentRequest {
				name: Some("John Patr's deployment".to_string()),
				machine_type: Some(
					Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
				),
				deploy_on_push: Some(true),
				min_horizontal_scale: Some(1),
				max_horizontal_scale: Some(2),
				ports: {
					let mut map = BTreeMap::new();

					map.insert(
						StringifiedU16::new(3000),
						ExposedPortType::Http,
					);
					map.insert(StringifiedU16::new(8080), ExposedPortType::Tcp);

					Some(map)
				},
				environment_variables: {
					let mut map = BTreeMap::new();

					map.insert(
						"APP_PORT".to_string(),
						EnvironmentVariableValue::String("3000".to_string()),
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

					Some(map)
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

					Some(map)
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
					Some(map)
				},
			},
			&[
				Token::Struct {
					name: "UpdateDeploymentRequest",
					len: 11,
				},
				Token::Str("name"),
				Token::Some,
				Token::Str("John Patr's deployment"),
				Token::Str("machineType"),
				Token::Some,
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("deployOnPush"),
				Token::Some,
				Token::Bool(true),
				Token::Str("minHorizontalScale"),
				Token::Some,
				Token::U16(1),
				Token::Str("maxHorizontalScale"),
				Token::Some,
				Token::U16(2),
				Token::Str("ports"),
				Token::Some,
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
				Token::Some,
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
				Token::Some,
				Token::Map { len: Some(1) },
				Token::Str("/app/config.json"),
				Token::Str(
					"ZmRidWFzZ2RzZ2Fvc3VlYWdod2VoaGd3OGhndXdlZ2hlb2doZQ==",
				),
				Token::MapEnd,
				Token::Str("volumes"),
				Token::Some,
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
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<UpdateDeploymentRequest as ApiRequest>::Response>(
			(),
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(()),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::MapEnd,
			],
		)
	}
}
