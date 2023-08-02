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
#[typed_path("/auth/list-recovery-options")]
pub struct ListRecoveryOptionsPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListRecoveryOptionsRequest {
	pub user_id: String,
}

impl ApiRequest for ListRecoveryOptionsRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = false;

	type RequestPath = ListRecoveryOptionsPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ListRecoveryOptionsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListRecoveryOptionsResponse {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub recovery_phone_number: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub recovery_email: Option<String>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ListRecoveryOptionsRequest, ListRecoveryOptionsResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListRecoveryOptionsRequest {
				user_id: "johnpatr@gmail.com".to_string(),
			},
			&[
				Token::Struct {
					name: "ListRecoveryOptionsRequest",
					len: 1,
				},
				Token::Str("userId"),
				Token::Str("johnpatr@gmail.com"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_email() {
		assert_tokens(
			&ListRecoveryOptionsResponse {
				recovery_email: Some("johnpatr@gmail.com".to_string()),
				recovery_phone_number: None,
			},
			&[
				Token::Struct {
					name: "ListRecoveryOptionsResponse",
					len: 1,
				},
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_phone_number() {
		assert_tokens(
			&ListRecoveryOptionsResponse {
				recovery_email: None,
				recovery_phone_number: Some("+911234567890".to_string()),
			},
			&[
				Token::Struct {
					name: "ListRecoveryOptionsResponse",
					len: 1,
				},
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Str("+911234567890"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_email_and_phone_number() {
		assert_tokens(
			&ListRecoveryOptionsResponse {
				recovery_phone_number: Some("+911234567890".to_string()),
				recovery_email: Some("johnpatr@gmail.com".to_string()),
			},
			&[
				Token::Struct {
					name: "ListRecoveryOptionsResponse",
					len: 2,
				},
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Str("+911234567890"),
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_email() {
		assert_tokens(
			&ApiResponse::success(ListRecoveryOptionsResponse {
				recovery_email: Some("johnpatr@gmail.com".to_string()),
				recovery_phone_number: None,
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_phone_number() {
		assert_tokens(
			&ApiResponse::success(ListRecoveryOptionsResponse {
				recovery_email: None,
				recovery_phone_number: Some("+911234567890".to_string()),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Str("+911234567890"),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_email_and_phone_number() {
		assert_tokens(
			&ApiResponse::success(ListRecoveryOptionsResponse {
				recovery_phone_number: Some("+911234567890".to_string()),
				recovery_email: Some("johnpatr@gmail.com".to_string()),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("recoveryPhoneNumber"),
				Token::Some,
				Token::Str("+911234567890"),
				Token::Str("recoveryEmail"),
				Token::Some,
				Token::Str("johnpatr@gmail.com"),
				Token::MapEnd,
			],
		);
	}
}
