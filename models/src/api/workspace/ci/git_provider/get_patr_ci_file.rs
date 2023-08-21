use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{
	utils::{Base64String, Uuid},
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
#[typed_path("/workspace/:workspace_id/ci/git-provider/github/repo/:repo_id/patr-ci-file/:git_ref")]
pub struct GetPatrCiFilePath {
	pub workspace_id: Uuid,
	pub repo_id: String,
	pub git_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetPatrCiFileRequest {}

impl ApiRequest for GetPatrCiFileRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetPatrCiFilePath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetPatrCiFileResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetPatrCiFileResponse {
	pub file_content: Base64String,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{GetPatrCiFileRequest, GetPatrCiFileResponse};
	use crate::{utils::Base64String, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetPatrCiFileRequest {},
			&[
				Token::Struct {
					name: "GetPatrCiFileRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetPatrCiFileResponse {
				file_content: Base64String::from("hello".as_bytes()),
			},
			&[
				Token::Struct {
					name: "GetPatrCiFileResponse",
					len: 1,
				},
				Token::Str("fileContent"),
				Token::Str("aGVsbG8="),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetPatrCiFileResponse {
				file_content: Base64String::from("hello".as_bytes()),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("fileContent"),
				Token::Str("aGVsbG8="),
				Token::MapEnd,
			],
		);
	}
}
