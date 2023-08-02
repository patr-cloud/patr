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
#[typed_path("/workspace/:workspace_id/rbac/permission")]
pub struct ListAllPermissionsPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
	pub id: Uuid,
	pub name: String,
	pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListAllPermissionsRequest;

impl ApiRequest for ListAllPermissionsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListAllPermissionsPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ListAllPermissionsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListAllPermissionsResponse {
	pub permissions: Vec<Permission>,
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::{
		ListAllPermissionsRequest,
		ListAllPermissionsResponse,
		Permission,
	};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListAllPermissionsRequest,
			&[Token::UnitStruct {
				name: "ListAllPermissionsRequest",
			}],
		);
	}

	#[test]
	fn assert_permission_types() {
		assert_tokens(
			&Permission {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				name: "permission:test".to_string(),
				description: "a minimal description".to_string(),
			},
			&[
				Token::Struct {
					name: "Permission",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("permission:test"),
				Token::Str("description"),
				Token::Str("a minimal description"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListAllPermissionsResponse {
				permissions: vec![Permission {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "permission:test".to_string(),
					description: "a minimal description".to_string(),
				}],
			},
			&[
				Token::Struct {
					name: "ListAllPermissionsResponse",
					len: 1,
				},
				Token::Str("permissions"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "Permission",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("permission:test"),
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
			&ApiResponse::success(ListAllPermissionsResponse {
				permissions: vec![Permission {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "permission:test".to_string(),
					description: "a minimal description".to_string(),
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("permissions"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "Permission",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("permission:test"),
				Token::Str("description"),
				Token::Str("a minimal description"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
