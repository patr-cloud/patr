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
#[typed_path("/auth/email-valid")]
pub struct IsEmailValidPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IsEmailValidRequest {
	pub email: String,
}

impl ApiRequest for IsEmailValidRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = false;

	type RequestPath = IsEmailValidPath;
	type RequestQuery = Self;
	type RequestBody = ();
	type Response = IsEmailValidResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IsEmailValidResponse {
	pub available: bool,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{IsEmailValidRequest, IsEmailValidResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&IsEmailValidRequest {
				email: "johnpatr@gmail.com".to_string(),
			},
			&[
				Token::Struct {
					name: "IsEmailValidRequest",
					len: 1,
				},
				Token::Str("email"),
				Token::Str("johnpatr@gmail.com"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_true() {
		assert_tokens(
			&IsEmailValidResponse { available: true },
			&[
				Token::Struct {
					name: "IsEmailValidResponse",
					len: 1,
				},
				Token::Str("available"),
				Token::Bool(true),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_false() {
		assert_tokens(
			&IsEmailValidResponse { available: false },
			&[
				Token::Struct {
					name: "IsEmailValidResponse",
					len: 1,
				},
				Token::Str("available"),
				Token::Bool(false),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_true() {
		assert_tokens(
			&ApiResponse::success(IsEmailValidResponse { available: true }),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("available"),
				Token::Bool(true),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_false() {
		assert_tokens(
			&ApiResponse::success(IsEmailValidResponse { available: false }),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("available"),
				Token::Bool(false),
				Token::MapEnd,
			],
		);
	}
}
