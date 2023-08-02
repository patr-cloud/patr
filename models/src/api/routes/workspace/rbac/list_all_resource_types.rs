use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

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
#[typed_path("/workspace/:workspace_id/rbac/resource-type")]
pub struct ListAllResourceTypesPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceType {
	pub id: Uuid,
	pub name: String,
	pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListAllResourceTypesRequest;

impl ApiRequest for ListAllResourceTypesRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListAllResourceTypesPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ListAllResourceTypesResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListAllResourceTypesResponse {
	pub resource_types: Vec<ResourceType>,
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::{
		ListAllResourceTypesRequest,
		ListAllResourceTypesResponse,
		ResourceType,
	};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListAllResourceTypesRequest,
			&[Token::UnitStruct {
				name: "ListAllResourceTypesRequest",
			}],
		);
	}

	#[test]
	fn assert_resource_types() {
		assert_tokens(
			&ResourceType {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				name: "ResourceType:test".to_string(),
				description: "a minimal description".to_string(),
			},
			&[
				Token::Struct {
					name: "ResourceType",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("ResourceType:test"),
				Token::Str("description"),
				Token::Str("a minimal description"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListAllResourceTypesResponse {
				resource_types: vec![ResourceType {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "ResourceType:test".to_string(),
					description: "a minimal description".to_string(),
				}],
			},
			&[
				Token::Struct {
					name: "ListAllResourceTypesResponse",
					len: 1,
				},
				Token::Str("resourceTypes"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "ResourceType",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("ResourceType:test"),
				Token::Str("description"),
				Token::Str("a minimal description"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListAllResourceTypesResponse {
				resource_types: vec![ResourceType {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "ResourceType:test".to_string(),
					description: "a minimal description".to_string(),
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("resourceTypes"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "ResourceType",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("ResourceType:test"),
				Token::Str("description"),
				Token::Str("a minimal description"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
