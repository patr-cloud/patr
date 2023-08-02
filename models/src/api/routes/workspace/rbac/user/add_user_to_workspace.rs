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
#[typed_path("/workspace/:workspace_id/rbac/user/:user_id")]
pub struct AddUserToWorkspacePath {
	pub workspace_id: Uuid,
	pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddUserToWorkspaceRequest {
	pub roles: Vec<Uuid>,
}

impl ApiRequest for AddUserToWorkspaceRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = AddUserToWorkspacePath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::AddUserToWorkspaceRequest;
	use crate::{utils::Uuid, ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types_add_user_to_workspace() {
		assert_tokens(
			&AddUserToWorkspaceRequest {
				roles: vec![
					Uuid::parse_str("2aef1a631ded45eb9170dc2166b30867")
						.unwrap(),
					Uuid::parse_str("2aef1c631ded45eb9170dc2166b30867")
						.unwrap(),
				],
			},
			&[
				Token::Struct {
					name: "AddUserToWorkspaceRequest",
					len: 1,
				},
				Token::Str("roles"),
				Token::Seq { len: Some(2) },
				Token::Str("2aef1a631ded45eb9170dc2166b30867"),
				Token::Str("2aef1c631ded45eb9170dc2166b30867"),
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<
			<AddUserToWorkspaceRequest as ApiRequest>::Response,
		>(());
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
