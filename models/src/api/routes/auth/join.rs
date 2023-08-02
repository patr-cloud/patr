use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{utils::Uuid, ApiRequest};

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
#[typed_path("/auth/join")]
pub struct CompleteSignUpPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSignUpRequest {
	pub username: String,
	pub verification_token: String,
}

impl ApiRequest for CompleteSignUpRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = false;

	type RequestPath = CompleteSignUpPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = CompleteSignUpResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSignUpResponse {
	pub access_token: String,
	pub refresh_token: Uuid,
	pub login_id: Uuid,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{CompleteSignUpRequest, CompleteSignUpResponse};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&CompleteSignUpRequest {
				username: "john-patr".to_string(),
				verification_token: "069-069".to_string(),
			},
			&[
				Token::Struct {
					name: "CompleteSignUpRequest",
					len: 2,
				},
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("verificationToken"),
				Token::Str("069-069"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		const ACCESS_TOKEN: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJodHRwczovL2FwaS5wYXRyLmNsb3VkIiwiYXVkIjoiaHR0cHM6Ly8qLnBhdHIuY2xvdWQiLCJpYXQiOjE2MzUyMzM2NDg2MzAsInR5cCI6ImFjY2Vzc1Rva2VuIiwiZXhwIjoxNjM1NDkyODQ4NjMwLCJvcmdzIjp7IjlhNDY2MmE3NTIzZDQ3OTFiMzJlMTAxOTY2MjQ1Njc1Ijp7ImlzU3VwZXJBZG1pbiI6dHJ1ZSwicmVzb3VyY2VzIjp7fSwicmVzb3VyY2VUeXBlcyI6e319fSwibG9naW5JZCI6ImYwMjliMjE1OWEyNjQ2MjU4MmExNDJjYmMzMGU2NTEyIiwidXNlciI6eyJpZCI6WzExLDE3OSwxNzEsMzEsNDUsNjQsNzcsMTg1LDE1Myw4MCwyNTMsMjksMzgsMzEsMTU1LDE2NV0sInVzZXJuYW1lIjoicmFrc2hpdGgtcmF2aSIsImZpcnN0TmFtZSI6IlJha3NoaXRoIiwibGFzdE5hbWUiOiJSYXZpIiwiY3JlYXRlZCI6MTYzNDg5MTE0NTMxNX19.L_xFtH-gN8AjOVwSnz4ruh3gAUgr94DwML2pIdrwMzc";
		assert_tokens(
			&CompleteSignUpResponse {
				access_token: ACCESS_TOKEN.to_string(),
				login_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				refresh_token: Uuid::parse_str(
					"f029b2159a26462582a142cbc30e6512",
				)
				.unwrap(),
			},
			&[
				Token::Struct {
					name: "CompleteSignUpResponse",
					len: 3,
				},
				Token::Str("accessToken"),
				Token::Str(ACCESS_TOKEN),
				Token::Str("refreshToken"),
				Token::Str("f029b2159a26462582a142cbc30e6512"),
				Token::Str("loginId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		const ACCESS_TOKEN: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJodHRwczovL2FwaS5wYXRyLmNsb3VkIiwiYXVkIjoiaHR0cHM6Ly8qLnBhdHIuY2xvdWQiLCJpYXQiOjE2MzUyMzM2NDg2MzAsInR5cCI6ImFjY2Vzc1Rva2VuIiwiZXhwIjoxNjM1NDkyODQ4NjMwLCJvcmdzIjp7IjlhNDY2MmE3NTIzZDQ3OTFiMzJlMTAxOTY2MjQ1Njc1Ijp7ImlzU3VwZXJBZG1pbiI6dHJ1ZSwicmVzb3VyY2VzIjp7fSwicmVzb3VyY2VUeXBlcyI6e319fSwibG9naW5JZCI6ImYwMjliMjE1OWEyNjQ2MjU4MmExNDJjYmMzMGU2NTEyIiwidXNlciI6eyJpZCI6WzExLDE3OSwxNzEsMzEsNDUsNjQsNzcsMTg1LDE1Myw4MCwyNTMsMjksMzgsMzEsMTU1LDE2NV0sInVzZXJuYW1lIjoicmFrc2hpdGgtcmF2aSIsImZpcnN0TmFtZSI6IlJha3NoaXRoIiwibGFzdE5hbWUiOiJSYXZpIiwiY3JlYXRlZCI6MTYzNDg5MTE0NTMxNX19.L_xFtH-gN8AjOVwSnz4ruh3gAUgr94DwML2pIdrwMzc";
		assert_tokens(
			&ApiResponse::success(CompleteSignUpResponse {
				access_token: ACCESS_TOKEN.to_string(),
				login_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				refresh_token: Uuid::parse_str(
					"f029b2159a26462582a142cbc30e6512",
				)
				.unwrap(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("accessToken"),
				Token::Str(ACCESS_TOKEN),
				Token::Str("refreshToken"),
				Token::Str("f029b2159a26462582a142cbc30e6512"),
				Token::Str("loginId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::MapEnd,
			],
		);
	}
}
