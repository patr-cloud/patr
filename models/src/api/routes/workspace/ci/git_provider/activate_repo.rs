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
	"/workspace/:workspace_id/ci/git-provider/github/repo/:repo_id/activate"
)]
pub struct ActivateRepoPath {
	pub workspace_id: Uuid,
	pub repo_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActivateRepoRequest {
	pub runner_id: Uuid,
}

impl ApiRequest for ActivateRepoRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = ActivateRepoPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::ActivateRepoRequest;
	use crate::{utils::Uuid, ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ActivateRepoRequest {
				runner_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "ActivateRepoRequest",
					len: 1,
				},
				Token::Str("runnerId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<ActivateRepoRequest as ApiRequest>::Response>(());
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
