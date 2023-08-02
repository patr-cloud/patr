use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{utils::Uuid, ApiRequest};

#[derive(
	Eq,
	Ord,
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
#[typed_path("/workspace/:workspace_id/billing/payment-method")]
pub struct AddPaymentMethodPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddPaymentMethodRequest;

impl ApiRequest for AddPaymentMethodRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = AddPaymentMethodPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = AddPaymentMethodResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddPaymentMethodResponse {
	pub client_secret: String,
	pub payment_intent_id: String,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{AddPaymentMethodRequest, AddPaymentMethodResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&AddPaymentMethodRequest,
			&[Token::UnitStruct {
				name: "AddPaymentMethodRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&AddPaymentMethodResponse {
				client_secret: "seti_1L8ig12eZvKYlo2C6NWlyQBS_secret_LqPVJK3X8GcqrDwUdBnXCeN3B1qIPoM".to_string(),
				payment_intent_id: "test-payment-id".to_string(),
			},
			&[
				Token::Struct {
					name: "AddPaymentMethodResponse",
					len: 2,
				},
				Token::Str("clientSecret"),
				Token::Str("seti_1L8ig12eZvKYlo2C6NWlyQBS_secret_LqPVJK3X8GcqrDwUdBnXCeN3B1qIPoM"),
				Token::Str("paymentIntentId"),
				Token::Str("test-payment-id"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(AddPaymentMethodResponse {
				client_secret: "seti_1L8ig12eZvKYlo2C6NWlyQBS_secret_LqPVJK3X8GcqrDwUdBnXCeN3B1qIPoM".to_string(),
				payment_intent_id: "test-payment-id".to_string(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("clientSecret"),
				Token::Str("seti_1L8ig12eZvKYlo2C6NWlyQBS_secret_LqPVJK3X8GcqrDwUdBnXCeN3B1qIPoM"),
				Token::Str("paymentIntentId"),
				Token::Str("test-payment-id"),
				Token::MapEnd,
			],
		);
	}
}
