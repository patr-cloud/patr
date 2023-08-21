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
pub struct RemoveUserFromWorkspacePath {
	pub workspace_id: Uuid,
	pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoveUserFromWorkspaceRequest {}

impl ApiRequest for RemoveUserFromWorkspaceRequest {
	const METHOD: Method = Method::DELETE;
	const IS_PROTECTED: bool = true;

	type RequestPath = RemoveUserFromWorkspacePath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::RemoveUserFromWorkspaceRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types_add_user_to_workspace() {
		assert_tokens(
			&RemoveUserFromWorkspaceRequest {},
			&[
				Token::Struct {
					name: "RemoveUserFromWorkspaceRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<
			<RemoveUserFromWorkspaceRequest as ApiRequest>::Response,
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
