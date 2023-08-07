use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::ApiRequest;

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
#[typed_path("/auth/oauth/github/authorize")]
pub struct GithubAuthorizePath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GithubAuthorizeRequest {}

impl ApiRequest for GithubAuthorizeRequest {
	const IS_PROTECTED: bool = false;
	const METHOD: Method = Method::POST;

	type RequestPath = GithubAuthorizePath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GithubAuthorizeResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GithubAuthorizeResponse {
	pub oauth_url: String,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{GithubAuthorizeRequest, GithubAuthorizeResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GithubAuthorizeRequest {},
			&[
				Token::Struct {
					name: "GithubAuthorizeRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GithubAuthorizeResponse {
				oauth_url: "https://github.com/login/oauth/authorize"
					.to_string(),
			},
			&[
				Token::Struct {
					name: "GithubAuthorizeResponse",
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
			&ApiResponse::success(GithubAuthorizeResponse {
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
