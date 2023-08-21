use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{utils::Uuid, ApiRequest};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentMachineType {
	pub id: Uuid,
	pub cpu_count: i16,
	pub memory_count: i32,
}

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
#[typed_path("/workspace/:workspace_id/infrastructure/machine-type")]
pub struct ListAllDeploymentMachineTypesPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListAllDeploymentMachineTypesRequest {}

impl ApiRequest for ListAllDeploymentMachineTypesRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListAllDeploymentMachineTypesPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ListAllDeploymentMachineTypesResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListAllDeploymentMachineTypesResponse {
	pub machine_types: Vec<DeploymentMachineType>,
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::{
		DeploymentMachineType,
		ListAllDeploymentMachineTypesRequest,
		ListAllDeploymentMachineTypesResponse,
	};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListAllDeploymentMachineTypesRequest {},
			&[
				Token::Struct {
					name: "ListAllDeploymentMachineTypesRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_deployment_machine_types() {
		assert_tokens(
			&DeploymentMachineType {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				cpu_count: 4,
				memory_count: 32,
			},
			&[
				Token::Struct {
					name: "DeploymentMachineType",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("cpuCount"),
				Token::I16(4),
				Token::Str("memoryCount"),
				Token::I32(32),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListAllDeploymentMachineTypesResponse {
				machine_types: vec![DeploymentMachineType {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					cpu_count: 2,
					memory_count: 16,
				}],
			},
			&[
				Token::Struct {
					name: "ListAllDeploymentMachineTypesResponse",
					len: 1,
				},
				Token::Str("machineTypes"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "DeploymentMachineType",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("cpuCount"),
				Token::I16(2),
				Token::Str("memoryCount"),
				Token::I32(16),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListAllDeploymentMachineTypesResponse {
				machine_types: vec![DeploymentMachineType {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					cpu_count: 2,
					memory_count: 16,
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("machineTypes"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "DeploymentMachineType",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("cpuCount"),
				Token::I16(2),
				Token::Str("memoryCount"),
				Token::I32(16),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
