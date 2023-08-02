use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::TotalAmount;
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
#[typed_path("/workspace/:workspace_id/billing/get-current-usage")]
pub struct GetCurrentUsagePath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetCurrentUsageRequest;

impl ApiRequest for GetCurrentUsageRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetCurrentUsagePath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetCurrentUsageResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetCurrentUsageResponse {
	#[serde(flatten)]
	pub current_usage: TotalAmount,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{GetCurrentUsageRequest, GetCurrentUsageResponse};
	use crate::{models::workspace::billing::TotalAmount, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetCurrentUsageRequest,
			&[Token::UnitStruct {
				name: "GetCurrentUsageRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetCurrentUsageResponse {
				current_usage: TotalAmount::CreditsLeft(5000),
			},
			&[
				Token::Map { len: None },
				Token::Str("creditsLeft"),
				Token::U64(5000),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetCurrentUsageResponse {
				current_usage: TotalAmount::NeedToPay(4000),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("needToPay"),
				Token::U64(4000),
				Token::MapEnd,
			],
		);
	}
}
