use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::BuildMachineType;
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
#[typed_path("/workspace/:workspace_id/ci/build-machine-type")]
pub struct ListAllBuildMachineTypesPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListAllBuildMachineTypesRequest;

impl ApiRequest for ListAllBuildMachineTypesRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListAllBuildMachineTypesPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ListAllBuildMachineTypesResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListAllBuildMachineTypesResponse {
	pub build_machine_types: Vec<BuildMachineType>,
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::{
		BuildMachineType,
		ListAllBuildMachineTypesRequest,
		ListAllBuildMachineTypesResponse,
	};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListAllBuildMachineTypesRequest,
			&[Token::UnitStruct {
				name: "ListAllBuildMachineTypesRequest",
			}],
		);
	}

	#[test]
	fn assert_deployment_machine_types() {
		assert_tokens(
			&BuildMachineType {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				cpu: 1,
				ram: 2,
				volume: 3,
			},
			&[
				Token::Struct {
					name: "BuildMachineType",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("cpu"),
				Token::I32(1),
				Token::Str("ram"),
				Token::I32(2),
				Token::Str("volume"),
				Token::I32(3),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListAllBuildMachineTypesResponse {
				build_machine_types: vec![BuildMachineType {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					cpu: 1,
					ram: 2,
					volume: 3,
				}],
			},
			&[
				Token::Struct {
					name: "ListAllBuildMachineTypesResponse",
					len: 1,
				},
				Token::Str("buildMachineTypes"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "BuildMachineType",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("cpu"),
				Token::I32(1),
				Token::Str("ram"),
				Token::I32(2),
				Token::Str("volume"),
				Token::I32(3),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListAllBuildMachineTypesResponse {
				build_machine_types: vec![BuildMachineType {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					cpu: 1,
					ram: 2,
					volume: 3,
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("buildMachineTypes"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "BuildMachineType",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("cpu"),
				Token::I32(1),
				Token::Str("ram"),
				Token::I32(2),
				Token::Str("volume"),
				Token::I32(3),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
