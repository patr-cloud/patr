use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{utils::Uuid, ApiRequest};

#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Default,
	TypedPath,
	Serialize,
	Deserialize,
)]
#[typed_path("/user/api-token/:token_id/regenerate")]
pub struct RegenerateApiTokenPath {
	pub token_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegenerateApiTokenRequest;

impl ApiRequest for RegenerateApiTokenRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = RegenerateApiTokenPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = RegenerateApiTokenResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegenerateApiTokenResponse {
	pub token: String,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{RegenerateApiTokenRequest, RegenerateApiTokenResponse};
	use crate::ApiResponse;

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&RegenerateApiTokenRequest,
			&[Token::UnitStruct {
				name: "RegenerateApiTokenRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&RegenerateApiTokenResponse {
				token: "api-token".to_string(),
			},
			&[
				Token::Struct {
					name: "RegenerateApiTokenResponse",
					len: 1,
				},
				Token::Str("token"),
				Token::Str("api-token"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(RegenerateApiTokenResponse {
				token: "api-token".to_string(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("token"),
				Token::Str("api-token"),
				Token::MapEnd,
			],
		);
	}
}
