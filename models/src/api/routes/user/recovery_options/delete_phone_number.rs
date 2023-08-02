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
#[typed_path("/user/delete-phone-number")]
pub struct DeletePhoneNumberPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeletePhoneNumberRequest {
	pub country_code: String,
	pub phone_number: String,
}

impl ApiRequest for DeletePhoneNumberRequest {
	const METHOD: Method = Method::DELETE;
	const IS_PROTECTED: bool = true;

	type RequestPath = DeletePhoneNumberPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::DeletePhoneNumberRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&DeletePhoneNumberRequest {
				country_code: "IN".to_string(),
				phone_number: "1234567890".to_string(),
			},
			&[
				Token::Struct {
					name: "DeletePhoneNumberRequest",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<DeletePhoneNumberRequest as ApiRequest>::Response>(
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
