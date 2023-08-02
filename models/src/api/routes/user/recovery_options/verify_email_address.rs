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
#[typed_path("/user/verify-phone-number")]
pub struct VerifyPersonalEmailPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerifyPersonalEmailRequest {
	pub email: String,
	pub verification_token: String,
}

impl ApiRequest for VerifyPersonalEmailRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = VerifyPersonalEmailPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::VerifyPersonalEmailRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&VerifyPersonalEmailRequest {
				email: "johnpatr@gmail.com".to_string(),
				verification_token: "069-069".to_string(),
			},
			&[
				Token::Struct {
					name: "VerifyPersonalEmailRequest",
					len: 2,
				},
				Token::Str("email"),
				Token::Str("johnpatr@gmail.com"),
				Token::Str("verificationToken"),
				Token::Str("069-069"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<
			<VerifyPersonalEmailRequest as ApiRequest>::Response,
		>(());
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
