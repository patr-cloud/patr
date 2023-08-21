use std::collections::BTreeMap;

use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{models::workspace::WorkspacePermission, utils::Uuid, ApiRequest};

#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Default,
	TypedPath,
	Serialize,
	Deserialize,
)]
#[typed_path("/user/api-token/:token_id/permission")]
pub struct ListApiTokenPermissionsPath {
	pub token_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListApiTokenPermissionsRequest;

impl ApiRequest for ListApiTokenPermissionsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListApiTokenPermissionsPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ListApiTokenPermissionsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListApiTokenPermissionsResponse {
	pub permissions: BTreeMap<Uuid, WorkspacePermission>,
}

#[cfg(test)]
mod test {
	use std::collections::BTreeMap;

	use serde_test::{assert_tokens, Token};

	use super::{
		ListApiTokenPermissionsRequest,
		ListApiTokenPermissionsResponse,
	};
	use crate::{
		models::workspace::WorkspacePermission,
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListApiTokenPermissionsRequest,
			&[Token::UnitStruct {
				name: "ListApiTokenPermissionsRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListApiTokenPermissionsResponse {
				permissions: {
					let mut map = BTreeMap::new();

					map.insert(
						Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						WorkspacePermission {
							is_super_admin: true,
							resource_permissions: BTreeMap::new(),
							resource_type_permissions: BTreeMap::new(),
						},
					);

					map
				},
			},
			&[
				Token::Struct {
					name: "ListApiTokenPermissionsResponse",
					len: 1,
				},
				Token::Str("permissions"),
				Token::Map { len: Some(1) },
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Struct {
					name: "WorkspacePermission",
					len: 3,
				},
				Token::Str("isSuperAdmin"),
				Token::Bool(true),
				Token::Str("resourcePermissions"),
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::Str("resourceTypePermissions"),
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::StructEnd,
				Token::MapEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListApiTokenPermissionsResponse {
				permissions: {
					let mut map = BTreeMap::new();

					map.insert(
						Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						WorkspacePermission {
							is_super_admin: true,
							resource_permissions: BTreeMap::new(),
							resource_type_permissions: BTreeMap::new(),
						},
					);

					map
				},
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("permissions"),
				Token::Map { len: Some(1) },
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Struct {
					name: "WorkspacePermission",
					len: 3,
				},
				Token::Str("isSuperAdmin"),
				Token::Bool(true),
				Token::Str("resourcePermissions"),
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::Str("resourceTypePermissions"),
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::StructEnd,
				Token::MapEnd,
				Token::MapEnd,
			],
		);
	}
}
