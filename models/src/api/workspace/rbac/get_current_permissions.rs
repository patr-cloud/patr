use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{models::workspace::WorkspacePermission, utils::Uuid, ApiRequest};

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
#[typed_path("/workspace/:workspace_id/rbac/current-permissions")]
pub struct GetCurrentPermissionsPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetCurrentPermissionsRequest;

impl ApiRequest for GetCurrentPermissionsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetCurrentPermissionsPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetCurrentPermissionsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetCurrentPermissionsResponse {
	#[serde(flatten)]
	pub permissions: WorkspacePermission,
}

#[cfg(test)]
mod test {
	use std::collections::{BTreeMap, BTreeSet};

	use serde_test::{assert_tokens, Token};

	use super::{GetCurrentPermissionsRequest, GetCurrentPermissionsResponse};
	use crate::{
		models::workspace::WorkspacePermission,
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetCurrentPermissionsRequest,
			&[Token::UnitStruct {
				name: "GetCurrentPermissionsRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetCurrentPermissionsResponse {
				permissions: WorkspacePermission {
					is_super_admin: true,
					resource_permissions: {
						let mut map = BTreeMap::new();

						map.insert(
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
								.unwrap(),
							BTreeSet::from([
								Uuid::parse_str(
									"2aef18631ded45eb9170dc2166b30868",
								)
								.unwrap(),
								Uuid::parse_str(
									"2aef18631ded45eb9170dc2166b30869",
								)
								.unwrap(),
							]),
						);

						map
					},
					resource_type_permissions: {
						let mut map = BTreeMap::new();

						map.insert(
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30877")
								.unwrap(),
							BTreeSet::from([
								Uuid::parse_str(
									"2aef18631ded45eb9170dc2166b30878",
								)
								.unwrap(),
								Uuid::parse_str(
									"2aef18631ded45eb9170dc2166b30879",
								)
								.unwrap(),
							]),
						);

						map
					},
				},
			},
			&[
				Token::Map { len: None },
				Token::Str("isSuperAdmin"),
				Token::Bool(true),
				Token::Str("resourcePermissions"),
				Token::Map { len: Some(1) },
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Seq { len: Some(2) },
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::SeqEnd,
				Token::MapEnd,
				Token::Str("resourceTypePermissions"),
				Token::Map { len: Some(1) },
				Token::Str("2aef18631ded45eb9170dc2166b30877"),
				Token::Seq { len: Some(2) },
				Token::Str("2aef18631ded45eb9170dc2166b30878"),
				Token::Str("2aef18631ded45eb9170dc2166b30879"),
				Token::SeqEnd,
				Token::MapEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetCurrentPermissionsResponse {
				permissions: WorkspacePermission {
					is_super_admin: true,
					resource_permissions: {
						let mut map = BTreeMap::new();

						map.insert(
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
								.unwrap(),
							BTreeSet::from([
								Uuid::parse_str(
									"2aef18631ded45eb9170dc2166b30868",
								)
								.unwrap(),
								Uuid::parse_str(
									"2aef18631ded45eb9170dc2166b30869",
								)
								.unwrap(),
							]),
						);

						map
					},
					resource_type_permissions: {
						let mut map = BTreeMap::new();

						map.insert(
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30877")
								.unwrap(),
							BTreeSet::from([
								Uuid::parse_str(
									"2aef18631ded45eb9170dc2166b30878",
								)
								.unwrap(),
								Uuid::parse_str(
									"2aef18631ded45eb9170dc2166b30879",
								)
								.unwrap(),
							]),
						);

						map
					},
				},
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("isSuperAdmin"),
				Token::Bool(true),
				Token::Str("resourcePermissions"),
				Token::Map { len: Some(1) },
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Seq { len: Some(2) },
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::SeqEnd,
				Token::MapEnd,
				Token::Str("resourceTypePermissions"),
				Token::Map { len: Some(1) },
				Token::Str("2aef18631ded45eb9170dc2166b30877"),
				Token::Seq { len: Some(2) },
				Token::Str("2aef18631ded45eb9170dc2166b30878"),
				Token::Str("2aef18631ded45eb9170dc2166b30879"),
				Token::SeqEnd,
				Token::MapEnd,
				Token::MapEnd,
			],
		);
	}
}
