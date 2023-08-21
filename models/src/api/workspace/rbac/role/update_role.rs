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
#[typed_path("/workspace/:workspace_id/rbac/role/:role_id")]
pub struct UpdateRolePath {
	pub workspace_id: Uuid,
	pub role_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleRequest {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub resource_permissions: Option<BTreeMap<Uuid, Vec<Uuid>>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub resource_type_permissions: Option<BTreeMap<Uuid, Vec<Uuid>>>,
}

impl ApiRequest for UpdateRoleRequest {
	const METHOD: Method = Method::PUT;
	const IS_PROTECTED: bool = true;

	type RequestPath = UpdateRolePath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ();
}

#[cfg(test)]
mod test {
	use std::collections::BTreeMap;

	use serde_test::{assert_tokens, Token};

	use super::UpdateRoleRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&UpdateRoleRequest {
				name: Some("Admin".to_string()),
				description: Some("Administrator".to_string()),
				resource_permissions: Some(BTreeMap::new()),
				resource_type_permissions: Some(BTreeMap::new()),
			},
			&[
				Token::Struct {
					name: "UpdateRoleRequest",
					len: 4,
				},
				Token::Str("name"),
				Token::Some,
				Token::Str("Admin"),
				Token::Str("description"),
				Token::Some,
				Token::Str("Administrator"),
				Token::Str("resourcePermissions"),
				Token::Some,
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::Str("resourceTypePermissions"),
				Token::Some,
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<UpdateRoleRequest as ApiRequest>::Response>(());
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
		);
	}
}
