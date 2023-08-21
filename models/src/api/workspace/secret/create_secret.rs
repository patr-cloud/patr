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
#[typed_path("/workspace/:workspace_id/secret")]
pub struct CreateSecretInWorkspacePath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateSecretInWorkspaceRequest {
	pub name: String,
	pub value: String,
}

impl ApiRequest for CreateSecretInWorkspaceRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = CreateSecretInWorkspacePath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = CreateSecretInWorkspaceResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateSecretInWorkspaceResponse {
	pub id: Uuid,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		CreateSecretInWorkspaceRequest,
		CreateSecretInWorkspaceResponse,
	};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&CreateSecretInWorkspaceRequest {
				name: "test".to_string(),
				value: "asdf;lkj".to_string(),
			},
			&[
				Token::Struct {
					name: "CreateSecretInWorkspaceRequest",
					len: 2,
				},
				Token::Str("name"),
				Token::Str("test"),
				Token::Str("value"),
				Token::Str("asdf;lkj"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&CreateSecretInWorkspaceResponse {
				id: Uuid::parse_str("2bff18631ded45ec9270ec1265c40867")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "CreateSecretInWorkspaceResponse",
					len: 1,
				},
				Token::Str("id"),
				Token::Str("2bff18631ded45ec9270ec1265c40867"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response() {
		assert_tokens(
			&ApiResponse::success(CreateSecretInWorkspaceResponse {
				id: Uuid::parse_str("2bff18631ded45ec9270ec1265c40867")
					.unwrap(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2bff18631ded45ec9270ec1265c40867"),
				Token::MapEnd,
			],
		);
	}
}
