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
#[typed_path(
	"/workspace/:workspace_id/ci/git-provider/github/repo/:repo_id/deactivate"
)]
pub struct DeactivateRepoPath {
	pub workspace_id: Uuid,
	pub repo_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeactivateRepoRequest {}

impl ApiRequest for DeactivateRepoRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = DeactivateRepoPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::DeactivateRepoRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&DeactivateRepoRequest {},
			&[
				Token::Struct {
					name: "DeactivateRepoRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<DeactivateRepoRequest as ApiRequest>::Response>(
			(),
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
