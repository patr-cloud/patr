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
#[typed_path("/workspace/:workspace_id/billing/make-payment")]
pub struct MakePaymentPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MakePaymentRequest {
	pub amount: u64,
	pub payment_method_id: String,
}

impl ApiRequest for MakePaymentRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = MakePaymentPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = MakePaymentResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MakePaymentResponse {
	pub transaction_id: Uuid,
	pub client_secret: String,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{MakePaymentRequest, MakePaymentResponse};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&MakePaymentRequest {
				amount: 500,
				payment_method_id: String::from("pm_random_id"),
			},
			&[
				Token::Struct {
					name: "MakePaymentRequest",
					len: 2,
				},
				Token::Str("amount"),
				Token::U64(500),
				Token::Str("paymentMethodId"),
				Token::Str("pm_random_id"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&MakePaymentResponse {
				transaction_id: Uuid::parse_str(
					"d5727fb4-9e6b-43df-8a46-0c698340fffb",
				)
				.unwrap(),
				client_secret: String::from("client secret"),
			},
			&[
				Token::Struct {
					name: "MakePaymentResponse",
					len: 2,
				},
				Token::Str("transactionId"),
				Token::Str("d5727fb49e6b43df8a460c698340fffb"),
				Token::Str("clientSecret"),
				Token::Str("client secret"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(MakePaymentResponse {
				transaction_id: Uuid::parse_str(
					"d5727fb4-9e6b-43df-8a46-0c698340fffb",
				)
				.unwrap(),
				client_secret: String::from("client secret"),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("transactionId"),
				Token::Str("d5727fb49e6b43df8a460c698340fffb"),
				Token::Str("clientSecret"),
				Token::Str("client secret"),
				Token::MapEnd,
			],
		);
	}
}
