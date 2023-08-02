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
pub struct VerifyPhoneNumberPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerifyPhoneNumberRequest {
	pub country_code: String,
	pub phone_number: String,
	pub verification_token: String,
}

impl ApiRequest for VerifyPhoneNumberRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = VerifyPhoneNumberPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::VerifyPhoneNumberRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&VerifyPhoneNumberRequest {
				country_code: "IN".to_string(),
				phone_number: "1234567890".to_string(),
				verification_token: "069-069".to_string(),
			},
			&[
				Token::Struct {
					name: "VerifyPhoneNumberRequest",
					len: 3,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::Str("verificationToken"),
				Token::Str("069-069"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<VerifyPhoneNumberRequest as ApiRequest>::Response>(
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
