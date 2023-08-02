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
#[typed_path("/workspace/:workspace_id/ci/git-provider/github/repo/:repo_id/patr-ci-file")]
pub struct WritePatrCiFilePath {
	pub workspace_id: Uuid,
	pub repo_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WritePatrCiFileRequest {
	pub commit_message: String,
	pub parent_commit_sha: String,
	pub branch_name: String,
	pub ci_file_content: String,
}

impl ApiRequest for WritePatrCiFileRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = WritePatrCiFilePath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::WritePatrCiFileRequest;
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&WritePatrCiFileRequest {
				branch_name: "main".into(),
				commit_message: "update patr ci file".into(),
				parent_commit_sha: "121f71f0da".into(),
				ci_file_content: "aGVsbG8=".into(),
			},
			&[
				Token::Struct {
					name: "WritePatrCiFileRequest",
					len: 4,
				},
				Token::Str("commitMessage"),
				Token::Str("update patr ci file"),
				Token::Str("parentCommitSha"),
				Token::Str("121f71f0da"),
				Token::Str("branchName"),
				Token::Str("main"),
				Token::Str("ciFileContent"),
				Token::Str("aGVsbG8="),
				Token::StructEnd,
			],
		);
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
