use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

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
#[typed_path("/workspace/:workspace_id/rbac/role/:role_id/users")]
pub struct ListUsersForRolePath {
	pub workspace_id: Uuid,
	pub role_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersForRoleRequest;

impl ApiRequest for ListUsersForRoleRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListUsersForRolePath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListUsersForRoleResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListUsersForRoleResponse {
	pub users: Vec<Uuid>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ListUsersForRoleRequest, ListUsersForRoleResponse};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types_add_user_to_workspace() {
		assert_tokens(
			&ListUsersForRoleRequest,
			&[Token::UnitStruct {
				name: "ListUsersForRoleRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListUsersForRoleResponse {
				users: vec![
					Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
						.unwrap(),
					Uuid::parse_str("2aef18631ded45eb9170dc2166b30869")
						.unwrap(),
					Uuid::parse_str("2aef18631ded45eb9170dc2166b30870")
						.unwrap(),
				],
			},
			&[
				Token::Struct {
					name: "ListUsersForRoleResponse",
					len: 1,
				},
				Token::Str("users"),
				Token::Seq { len: Some(4) },
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::Str("2aef18631ded45eb9170dc2166b30870"),
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListUsersForRoleResponse {
				users: vec![
					Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
						.unwrap(),
					Uuid::parse_str("2aef18631ded45eb9170dc2166b30869")
						.unwrap(),
					Uuid::parse_str("2aef18631ded45eb9170dc2166b30870")
						.unwrap(),
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("users"),
				Token::Seq { len: Some(4) },
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::Str("2aef18631ded45eb9170dc2166b30870"),
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
