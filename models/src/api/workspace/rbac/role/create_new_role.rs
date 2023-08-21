use std::collections::BTreeMap;

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
#[typed_path("/workspaces/:workspace_id/rbac/role")]
pub struct CreateNewRolePath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateNewRoleRequest {
	pub name: String,
	#[serde(default, skip_serializing_if = "String::is_empty")]
	pub description: String,
	pub resource_permissions: BTreeMap<Uuid, Vec<Uuid>>,
	pub resource_type_permissions: BTreeMap<Uuid, Vec<Uuid>>,
}

impl ApiRequest for CreateNewRoleRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = CreateNewRolePath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = CreateNewRoleResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateNewRoleResponse {
	pub id: Uuid,
}

#[cfg(test)]
mod test {
	use std::collections::BTreeMap;

	use serde_test::{assert_tokens, Token};

	use super::{CreateNewRoleRequest, CreateNewRoleResponse};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&CreateNewRoleRequest {
				name: "Software Developer".to_string(),
				description: String::new(),
				resource_permissions: BTreeMap::new(),
				resource_type_permissions: BTreeMap::new(),
			},
			&[
				Token::Struct {
					name: "CreateNewRoleRequest",
					len: 3,
				},
				Token::Str("name"),
				Token::Str("Software Developer"),
				Token::Str("resourcePermissions"),
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::Str("resourceTypePermissions"),
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&CreateNewRoleResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "CreateNewRoleResponse",
					len: 1,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(CreateNewRoleResponse {
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
		);
	}
}
