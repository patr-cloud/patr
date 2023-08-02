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
#[typed_path("/auth/reset-password")]
pub struct ResetPasswordPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequest {
	pub user_id: String,
	pub verification_token: String,
	pub password: String,
}

impl ApiRequest for ResetPasswordRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = false;

	type RequestPath = ResetPasswordPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::ResetPasswordRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ResetPasswordRequest {
				user_id: String::from("john-patr"),
				verification_token: String::from("069-069"),
				password: String::from("hunter42"),
			},
			&[
				Token::Struct {
					name: "ResetPasswordRequest",
					len: 3,
				},
				Token::Str("userId"),
				Token::Str("john-patr"),
				Token::Str("verificationToken"),
				Token::Str("069-069"),
				Token::Str("password"),
				Token::Str("hunter42"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<ResetPasswordRequest as ApiRequest>::Response>(
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
