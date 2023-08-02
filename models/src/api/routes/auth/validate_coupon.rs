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
#[typed_path("/auth/coupon-valid")]
pub struct IsCouponValidPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IsCouponValidRequest {
	pub coupon: String,
}

impl ApiRequest for IsCouponValidRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = false;

	type RequestPath = IsCouponValidPath;
	type RequestQuery = Self;
	type RequestBody = ();
	type Response = IsCouponValidResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IsCouponValidResponse {
	pub valid: bool,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{IsCouponValidRequest, IsCouponValidResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&IsCouponValidRequest {
				coupon: "TRYPATR".to_string(),
			},
			&[
				Token::Struct {
					name: "IsCouponValidRequest",
					len: 1,
				},
				Token::Str("coupon"),
				Token::Str("TRYPATR"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_true() {
		assert_tokens(
			&IsCouponValidResponse { valid: true },
			&[
				Token::Struct {
					name: "IsCouponValidResponse",
					len: 1,
				},
				Token::Str("valid"),
				Token::Bool(true),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_false() {
		assert_tokens(
			&IsCouponValidResponse { valid: false },
			&[
				Token::Struct {
					name: "IsCouponValidResponse",
					len: 1,
				},
				Token::Str("valid"),
				Token::Bool(false),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_true() {
		assert_tokens(
			&ApiResponse::success(IsCouponValidResponse { valid: true }),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("valid"),
				Token::Bool(true),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_false() {
		assert_tokens(
			&ApiResponse::success(IsCouponValidResponse { valid: false }),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("valid"),
				Token::Bool(false),
				Token::MapEnd,
			],
		);
	}
}
