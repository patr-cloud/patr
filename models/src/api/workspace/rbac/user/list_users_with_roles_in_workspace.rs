use std::collections::BTreeMap;

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
#[typed_path("/workspace/:workspace_id/rbac/user")]
pub struct ListUsersWithRolesInWorkspacePath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersWithRolesInWorkspaceRequest {}

impl ApiRequest for ListUsersWithRolesInWorkspaceRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListUsersWithRolesInWorkspacePath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListUsersWithRolesInWorkspaceResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListUsersWithRolesInWorkspaceResponse {
	#[serde(flatten)]
	pub users: BTreeMap<Uuid, Vec<Uuid>>,
}

#[cfg(test)]
mod test {
	use std::collections::BTreeMap;

	use serde_test::{assert_tokens, Token};

	use super::{
		ListUsersWithRolesInWorkspaceRequest,
		ListUsersWithRolesInWorkspaceResponse,
	};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types_add_user_to_workspace() {
		assert_tokens(
			&ListUsersWithRolesInWorkspaceRequest {},
			&[
				Token::Struct {
					name: "ListUsersWithRolesInWorkspaceRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListUsersWithRolesInWorkspaceResponse {
				users: {
					let mut map = BTreeMap::new();

					map.insert(
						Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						vec![
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
								.unwrap(),
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
								.unwrap(),
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30869")
								.unwrap(),
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30870")
								.unwrap(),
						],
					);
					map.insert(
						Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
							.unwrap(),
						vec![
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30865")
								.unwrap(),
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30866")
								.unwrap(),
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30869")
								.unwrap(),
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30870")
								.unwrap(),
						],
					);

					map
				},
			},
			&[
				Token::Map { len: None },
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Seq { len: Some(4) },
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::Str("2aef18631ded45eb9170dc2166b30870"),
				Token::SeqEnd,
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Seq { len: Some(4) },
				Token::Str("2aef18631ded45eb9170dc2166b30865"),
				Token::Str("2aef18631ded45eb9170dc2166b30866"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::Str("2aef18631ded45eb9170dc2166b30870"),
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListUsersWithRolesInWorkspaceResponse {
				users: {
					let mut map = BTreeMap::new();

					map.insert(
						Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						vec![
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
								.unwrap(),
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
								.unwrap(),
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30869")
								.unwrap(),
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30870")
								.unwrap(),
						],
					);
					map.insert(
						Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
							.unwrap(),
						vec![
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30865")
								.unwrap(),
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30866")
								.unwrap(),
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30869")
								.unwrap(),
							Uuid::parse_str("2aef18631ded45eb9170dc2166b30870")
								.unwrap(),
						],
					);

					map
				},
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Seq { len: Some(4) },
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::Str("2aef18631ded45eb9170dc2166b30870"),
				Token::SeqEnd,
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Seq { len: Some(4) },
				Token::Str("2aef18631ded45eb9170dc2166b30865"),
				Token::Str("2aef18631ded45eb9170dc2166b30866"),
				Token::Str("2aef18631ded45eb9170dc2166b30869"),
				Token::Str("2aef18631ded45eb9170dc2166b30870"),
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
