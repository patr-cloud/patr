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
#[typed_path("/auth/sign-in")]
pub struct LoginPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
	pub user_id: String,
	pub password: String,
}

impl ApiRequest for LoginRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = false;

	type RequestPath = LoginPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = LoginResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
	pub access_token: String,
	pub refresh_token: Uuid,
	pub login_id: Uuid,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{LoginRequest, LoginResponse};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&LoginRequest {
				user_id: "john-patr".to_string(),
				password: "hunter42".to_string(),
			},
			&[
				Token::Struct {
					name: "LoginRequest",
					len: 2,
				},
				Token::Str("userId"),
				Token::Str("john-patr"),
				Token::Str("password"),
				Token::Str("hunter42"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		const ACCESS_TOKEN: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJodHRwczovL2FwaS5wYXRyLmNsb3VkIiwiYXVkIjoiaHR0cHM6Ly8qLnBhdHIuY2xvdWQiLCJpYXQiOjE2MzUyMzM2NDg2MzAsInR5cCI6ImFjY2Vzc1Rva2VuIiwiZXhwIjoxNjM1NDkyODQ4NjMwLCJvcmdzIjp7IjlhNDY2MmE3NTIzZDQ3OTFiMzJlMTAxOTY2MjQ1Njc1Ijp7ImlzU3VwZXJBZG1pbiI6dHJ1ZSwicmVzb3VyY2VzIjp7fSwicmVzb3VyY2VUeXBlcyI6e319fSwibG9naW5JZCI6ImYwMjliMjE1OWEyNjQ2MjU4MmExNDJjYmMzMGU2NTEyIiwidXNlciI6eyJpZCI6WzExLDE3OSwxNzEsMzEsNDUsNjQsNzcsMTg1LDE1Myw4MCwyNTMsMjksMzgsMzEsMTU1LDE2NV0sInVzZXJuYW1lIjoicmFrc2hpdGgtcmF2aSIsImZpcnN0TmFtZSI6IlJha3NoaXRoIiwibGFzdE5hbWUiOiJSYXZpIiwiY3JlYXRlZCI6MTYzNDg5MTE0NTMxNX19.L_xFtH-gN8AjOVwSnz4ruh3gAUgr94DwML2pIdrwMzc";
		assert_tokens(
			&LoginResponse {
				access_token: ACCESS_TOKEN.to_string(),
				refresh_token: Uuid::parse_str(
					"f029b2159a26462582a142cbc30e6512",
				)
				.unwrap(),
				login_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "LoginResponse",
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
			&ApiResponse::success(LoginResponse {
				access_token: ACCESS_TOKEN.to_string(),
				refresh_token: Uuid::parse_str(
					"f029b2159a26462582a142cbc30e6512",
				)
				.unwrap(),
				login_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
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
