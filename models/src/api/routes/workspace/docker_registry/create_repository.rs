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
#[typed_path("/workspace/:workspace_id/docker-registry")]
pub struct CreateDockerRepositoryPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateDockerRepositoryRequest {
	pub repository: String,
}

impl ApiRequest for CreateDockerRepositoryRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = CreateDockerRepositoryPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = CreateDockerRepositoryResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateDockerRepositoryResponse {
	pub id: Uuid,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		CreateDockerRepositoryRequest,
		CreateDockerRepositoryResponse,
	};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&CreateDockerRepositoryRequest {
				repository: "test-repo".to_string(),
			},
			&[
				Token::Struct {
					name: "CreateDockerRepositoryRequest",
					len: 1,
				},
				Token::Str("repository"),
				Token::Str("test-repo"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&CreateDockerRepositoryResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "CreateDockerRepositoryResponse",
					len: 1,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(CreateDockerRepositoryResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::MapEnd,
			],
		);
	}
}
