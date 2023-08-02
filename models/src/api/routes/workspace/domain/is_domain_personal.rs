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
#[typed_path("/workspace/:workspace_id/is-domain-personal")]
pub struct IsDomainPersonalPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IsDomainPersonalRequest {
	pub domain: String,
}

impl ApiRequest for IsDomainPersonalRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = false;

	type RequestPath = IsDomainPersonalPath;
	type RequestQuery = Self;
	type RequestBody = ();
	type Response = IsDomainPersonalResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IsDomainPersonalResponse {
	pub personal: bool,
	pub is_used_by_others: bool,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{IsDomainPersonalRequest, IsDomainPersonalResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&IsDomainPersonalRequest {
				domain: "patr.cloud".to_string(),
			},
			&[
				Token::Struct {
					name: "IsDomainPersonalRequest",
					len: 1,
				},
				Token::Str("domain"),
				Token::Str("patr.cloud"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_true() {
		assert_tokens(
			&IsDomainPersonalResponse {
				personal: true,
				is_used_by_others: true,
			},
			&[
				Token::Struct {
					name: "IsDomainPersonalResponse",
					len: 2,
				},
				Token::Str("personal"),
				Token::Bool(true),
				Token::Str("isUsedByOthers"),
				Token::Bool(true),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types_false() {
		assert_tokens(
			&IsDomainPersonalResponse {
				personal: false,
				is_used_by_others: false,
			},
			&[
				Token::Struct {
					name: "IsDomainPersonalResponse",
					len: 2,
				},
				Token::Str("personal"),
				Token::Bool(false),
				Token::Str("isUsedByOthers"),
				Token::Bool(false),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_true() {
		assert_tokens(
			&ApiResponse::success(IsDomainPersonalResponse {
				personal: true,
				is_used_by_others: true,
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("personal"),
				Token::Bool(true),
				Token::Str("isUsedByOthers"),
				Token::Bool(true),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_false() {
		assert_tokens(
			&ApiResponse::success(IsDomainPersonalResponse {
				personal: false,
				is_used_by_others: false,
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("personal"),
				Token::Bool(false),
				Token::Str("isUsedByOthers"),
				Token::Bool(false),
				Token::MapEnd,
			],
		);
	}
}
