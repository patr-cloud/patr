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
#[typed_path("/user/update-recovery-phone")]
pub struct UpdateRecoveryPhoneNumberPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecoveryPhoneNumberRequest {
	pub recovery_phone_country_code: String,
	pub recovery_phone_number: String,
}

impl ApiRequest for UpdateRecoveryPhoneNumberRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = UpdateRecoveryPhoneNumberPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::UpdateRecoveryPhoneNumberRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&UpdateRecoveryPhoneNumberRequest {
				recovery_phone_country_code: "IN".to_string(),
				recovery_phone_number: "1234567890".to_string(),
			},
			&[
				Token::Struct {
					name: "UpdateRecoveryPhoneNumberRequest",
					len: 2,
				},
				Token::Str("recoveryPhoneCountryCode"),
				Token::Str("IN"),
				Token::Str("recoveryPhoneNumber"),
				Token::Str("1234567890"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<
			<UpdateRecoveryPhoneNumberRequest as ApiRequest>::Response,
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
