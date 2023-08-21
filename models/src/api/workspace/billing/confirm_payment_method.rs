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
#[typed_path("/workspace/:workspace_id/billing/confirm-payment-method")]
pub struct ConfirmPaymentMethodPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmPaymentMethodRequest {
	pub payment_method_id: String,
}

impl ApiRequest for ConfirmPaymentMethodRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = ConfirmPaymentMethodPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::ConfirmPaymentMethodRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ConfirmPaymentMethodRequest {
				payment_method_id: "pm_6K95KhSGEPBh7GrIsWVB4pyV".to_string(),
			},
			&[
				Token::Struct {
					name: "ConfirmPaymentMethodRequest",
					len: 1,
				},
				Token::Str("paymentIntentId"),
				Token::Str("pi_6K95KhSGEPBh7GrIsWVB4pyV"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<
			<ConfirmPaymentMethodRequest as ApiRequest>::Response,
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
