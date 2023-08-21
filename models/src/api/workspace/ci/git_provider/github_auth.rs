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
#[typed_path("/workspace/:workspace_id/ci/git-provider/github/auth")]
pub struct GithubAuthPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GithubAuthRequest {}

impl ApiRequest for GithubAuthRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GithubAuthPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GithubAuthResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GithubAuthResponse {
	pub oauth_url: String,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{GithubAuthRequest, GithubAuthResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GithubAuthRequest {},
			&[
				Token::Struct {
					name: "GithubAuthRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GithubAuthResponse {
				oauth_url: "https://github.com/login/oauth/authorize"
					.to_string(),
			},
			&[
				Token::Struct {
					name: "GithubAuthResponse",
					len: 1,
				},
				Token::Str("oauthUrl"),
				Token::Str("https://github.com/login/oauth/authorize"),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GithubAuthResponse {
				oauth_url: "https://github.com/login/oauth/authorize"
					.to_string(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("oauthUrl"),
				Token::Str("https://github.com/login/oauth/authorize"),
				Token::MapEnd,
			],
		);
	}
}
