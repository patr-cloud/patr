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
#[typed_path("/auth/resend-otp")]
pub struct ResendOtpPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResendOtpRequest {
	pub username: String,
	pub password: String,
}

impl ApiRequest for ResendOtpRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = false;

	type RequestPath = ResendOtpPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::ResendOtpRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ResendOtpRequest {
				username: String::from("john-patr"),
				password: String::from("hunter42"),
			},
			&[
				Token::Struct {
					name: "ResendOtpRequest",
					len: 2,
				},
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("password"),
				Token::Str("hunter42"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<ResendOtpRequest as ApiRequest>::Response>(());
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
