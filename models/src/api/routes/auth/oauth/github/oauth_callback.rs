use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{
	utils::{True, Uuid},
	ApiRequest,
};

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
#[typed_path("/auth/oauth/github/callback")]
pub struct GithubOAuthCallbackPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GithubOAuthCallbackRequest {
	pub code: String,
	pub state: String,
	pub username: Option<String>,
}

impl ApiRequest for GithubOAuthCallbackRequest {
	const IS_PROTECTED: bool = false;
	const METHOD: Method = Method::POST;

	type RequestPath = GithubOAuthCallbackPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = GithubOAuthCallbackResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum GithubOAuthCallbackResponse {
	#[serde(rename_all = "camelCase")]
	Login {
		access_token: String,
		refresh_token: Uuid,
		login_id: Uuid,
	},
	#[serde(rename_all = "camelCase")]
	SignUp { verification_required: True },
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{GithubOAuthCallbackRequest, GithubOAuthCallbackResponse};
	use crate::{
		utils::{True, Uuid},
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GithubOAuthCallbackRequest {
				code: "1232412412410".to_string(),
				state: "1242412415252".to_string(),
				username: Some("test".to_string()),
			},
			&[
				Token::Struct {
					name: "GithubOAuthCallbackRequest",
					len: 3,
				},
				Token::Str("code"),
				Token::Str("1232412412410"),
				Token::Str("state"),
				Token::Str("1242412415252"),
				Token::Str("username"),
				Token::Some,
				Token::Str("test"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_sign_up_types() {
		assert_tokens(
			&GithubOAuthCallbackResponse::SignUp {
				verification_required: True,
			},
			&[
				Token::Struct {
					name: "GithubOAuthCallbackResponse",
					len: 1,
				},
				Token::Str("verificationRequired"),
				Token::Bool(true),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_login_types() {
		assert_tokens(
			&GithubOAuthCallbackResponse::Login {
				access_token: "test".to_string(),
				refresh_token: Uuid::parse_str(
					"2aef18631ded45eb9170dc2166b30867",
				)
				.unwrap(),
				login_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "GithubOAuthCallbackResponse",
					len: 3,
				},
				Token::Str("accessToken"),
				Token::Str("test"),
				Token::Str("refreshToken"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("loginId"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_sign_up_types() {
		assert_tokens(
			&ApiResponse::success(GithubOAuthCallbackResponse::SignUp {
				verification_required: True,
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("verificationRequired"),
				Token::Bool(true),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_login_types() {
		assert_tokens(
			&ApiResponse::success(GithubOAuthCallbackResponse::Login {
				access_token: "test".to_string(),
				refresh_token: Uuid::parse_str(
					"2aef18631ded45eb9170dc2166b30867",
				)
				.unwrap(),
				login_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
					.unwrap(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("accessToken"),
				Token::Str("test"),
				Token::Str("refreshToken"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("loginId"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::MapEnd,
			],
		);
	}
}
