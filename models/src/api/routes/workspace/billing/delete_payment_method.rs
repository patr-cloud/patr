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
#[typed_path(
	"/workspace/:workspace_id/billing/payment-method/:payment_method_id"
)]
pub struct DeletePaymentMethodPath {
	pub workspace_id: Uuid,
	pub payment_method_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeletePaymentMethodRequest;

impl ApiRequest for DeletePaymentMethodRequest {
	const METHOD: Method = Method::DELETE;
	const IS_PROTECTED: bool = true;

	type RequestPath = DeletePaymentMethodPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::DeletePaymentMethodRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&DeletePaymentMethodRequest,
			&[Token::UnitStruct {
				name: "DeletePaymentMethodRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<
			<DeletePaymentMethodRequest as ApiRequest>::Response,
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
