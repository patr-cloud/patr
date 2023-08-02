use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{models::user::UserPhoneNumber, utils::Paginated, ApiRequest};

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
#[typed_path("/user/list-phone-numbers")]
pub struct ListPhoneNumbersPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListPhoneNumbersRequest;

impl ApiRequest for ListPhoneNumbersRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListPhoneNumbersPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListPhoneNumbersResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListPhoneNumbersResponse {
	pub recovery_phone_number: Option<UserPhoneNumber>,
	pub secondary_phone_numbers: Vec<UserPhoneNumber>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ListPhoneNumbersRequest, ListPhoneNumbersResponse};
	use crate::{models::user::UserPhoneNumber, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListPhoneNumbersRequest,
			&[Token::UnitStruct {
				name: "ListPhoneNumbersRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListPhoneNumbersResponse {
				recovery_phone_number: Some(UserPhoneNumber {
					country_code: "IN".to_string(),
					phone_number: "1234567890".to_string(),
				}),
				secondary_phone_numbers: vec![UserPhoneNumber {
					country_code: "IN".to_string(),
					phone_number: "2134567890".to_string(),
				}],
			},
			&[
				Token::Struct {
					name: "ListPhoneNumbersResponse",
					len: 2,
				},
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
				Token::Str("secondaryPhoneNumbers"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("2134567890"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListPhoneNumbersResponse {
				recovery_phone_number: Some(UserPhoneNumber {
					country_code: "IN".to_string(),
					phone_number: "1234567890".to_string(),
				}),
				secondary_phone_numbers: vec![UserPhoneNumber {
					country_code: "IN".to_string(),
					phone_number: "2134567890".to_string(),
				}],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
				Token::Str("secondaryPhoneNumbers"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "UserPhoneNumber",
					len: 2,
				},
				Token::Str("countryCode"),
				Token::Str("IN"),
				Token::Str("phoneNumber"),
				Token::Str("2134567890"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
