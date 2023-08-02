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
#[typed_path("/auth/username-valid")]
pub struct IsUsernameValidPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IsUsernameValidRequest {
	pub username: String,
}

impl ApiRequest for IsUsernameValidRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = false;

	type RequestPath = IsUsernameValidPath;
	type RequestQuery = Self;
	type RequestBody = ();
	type Response = IsUsernameValidResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IsUsernameValidResponse {
	pub available: bool,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{IsUsernameValidRequest, IsUsernameValidResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&IsUsernameValidRequest {
				username: "john-patr".to_string(),
			},
			&[
				Token::Struct {
					name: "IsUsernameValidRequest",
					len: 1,
				},
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_true() {
		assert_tokens(
			&IsUsernameValidResponse { available: true },
			&[
				Token::Struct {
					name: "IsUsernameValidResponse",
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
			&IsUsernameValidResponse { available: false },
			&[
				Token::Struct {
					name: "IsUsernameValidResponse",
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
			&ApiResponse::success(IsUsernameValidResponse { available: true }),
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
			&ApiResponse::success(IsUsernameValidResponse { available: false }),
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
