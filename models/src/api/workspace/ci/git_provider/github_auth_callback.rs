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
#[typed_path("/workspace/:workspace_id/ci/git-provider/github/auth-callback")]
pub struct GithubAuthCallbackPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GithubAuthCallbackRequest {
	pub code: String,
	pub state: String,
}

impl ApiRequest for GithubAuthCallbackRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = GithubAuthCallbackPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::GithubAuthCallbackRequest;
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GithubAuthCallbackRequest {
				code: "1232412412410".to_string(),
				state: "1242412415252".to_string(),
			},
			&[
				Token::Struct {
					name: "GithubAuthCallbackRequest",
					len: 2,
				},
				Token::Str("code"),
				Token::Str("1232412412410"),
				Token::Str("state"),
				Token::Str("1242412415252"),
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
