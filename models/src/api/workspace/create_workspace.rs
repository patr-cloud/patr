use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{utils::Uuid, ApiRequest};

#[derive(
	Eq,
	Ord,
	Copy,
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
#[typed_path("/workspace")]
pub struct CreateWorkspacePath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateNewWorkspaceRequest {
	pub workspace_name: String,
}

impl ApiRequest for CreateNewWorkspaceRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = CreateWorkspacePath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = CreateNewWorkspaceResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateNewWorkspaceResponse {
	pub workspace_id: Uuid,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{CreateNewWorkspaceRequest, CreateNewWorkspaceResponse};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&CreateNewWorkspaceRequest {
				workspace_name: "John Patr's Company".to_string(),
			},
			&[
				Token::Struct {
					name: "CreateNewWorkspaceRequest",
					len: 1,
				},
				Token::Str("workspaceName"),
				Token::Str("John Patr's Company"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&CreateNewWorkspaceResponse {
				workspace_id: Uuid::parse_str(
					"2aef18631ded45eb9170dc2166b30867",
				)
				.unwrap(),
			},
			&[
				Token::Struct {
					name: "CreateNewWorkspaceResponse",
					len: 1,
				},
				Token::Str("workspaceId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(CreateNewWorkspaceResponse {
				workspace_id: Uuid::parse_str(
					"2aef18631ded45eb9170dc2166b30867",
				)
				.unwrap(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("workspaceId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::MapEnd,
			],
		);
	}
}
