use std::collections::BTreeMap;

use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::Role;
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
#[typed_path("/workspaces/:workspace_id/rbac/role/:role_id")]
pub struct GetRoleDetailsPath {
	pub workspace_id: Uuid,
	pub role_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetRoleDetailsRequest {}

impl ApiRequest for GetRoleDetailsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetRoleDetailsPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetRoleDetailsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetRoleDetailsResponse {
	#[serde(flatten)]
	pub role: Role,
	pub resource_permissions: BTreeMap<Uuid, Vec<Uuid>>,
	pub resource_type_permissions: BTreeMap<Uuid, Vec<Uuid>>,
}

#[cfg(test)]
mod test {
	use std::collections::BTreeMap;

	use serde_test::{assert_tokens, Token};

	use super::{super::Role, GetRoleDetailsRequest, GetRoleDetailsResponse};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetRoleDetailsRequest {},
			&[
				Token::Struct {
					name: "GetRoleDetailsRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetRoleDetailsResponse {
				role: Role {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "Admin".to_string(),
					description: "Administrator".to_string(),
				},
				resource_permissions: BTreeMap::new(),
				resource_type_permissions: BTreeMap::new(),
			},
			&[
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("Admin"),
				Token::Str("description"),
				Token::Str("Administrator"),
				Token::Str("resourcePermissions"),
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::Str("resourceTypePermissions"),
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetRoleDetailsResponse {
				role: Role {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "Admin".to_string(),
					description: "Administrator".to_string(),
				},
				resource_permissions: BTreeMap::new(),
				resource_type_permissions: BTreeMap::new(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("Admin"),
				Token::Str("description"),
				Token::Str("Administrator"),
				Token::Str("resourcePermissions"),
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::Str("resourceTypePermissions"),
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::MapEnd,
			],
		);
	}
}
