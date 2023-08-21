use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::Role;
use crate::{
	utils::{Paginated, Uuid},
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
#[typed_path("/workspaces/:workspace_id/rbac/role")]
pub struct ListAllRolesPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListAllRolesRequest {}

impl ApiRequest for ListAllRolesRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListAllRolesPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListAllRolesResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListAllRolesResponse {
	pub roles: Vec<Role>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ListAllRolesRequest, ListAllRolesResponse, Role};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListAllRolesRequest {},
			&[
				Token::Struct {
					name: "ListAllRolesRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListAllRolesResponse {
				roles: vec![
					Role {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "Software Developer".to_string(),
						description: String::new(),
					},
					Role {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
							.unwrap(),
						name: "Software Tester".to_string(),
						description: String::new(),
					},
				],
			},
			&[
				Token::Struct {
					name: "ListAllRolesResponse",
					len: 1,
				},
				Token::Str("roles"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "Role",
					len: 2,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("Software Developer"),
				Token::StructEnd,
				Token::Struct {
					name: "Role",
					len: 2,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("name"),
				Token::Str("Software Tester"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListAllRolesResponse {
				roles: vec![
					Role {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "Software Developer".to_string(),
						description: String::new(),
					},
					Role {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
							.unwrap(),
						name: "Software Tester".to_string(),
						description: String::new(),
					},
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("roles"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "Role",
					len: 2,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("Software Developer"),
				Token::StructEnd,
				Token::Struct {
					name: "Role",
					len: 2,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("name"),
				Token::Str("Software Tester"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
