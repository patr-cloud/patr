use axum_extra::routing::TypedPath;
use reqwest::{
	header::{self, HeaderMap, HeaderValue},
	Method,
};
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
#[typed_path("/auth/access-token")]
pub struct RenewAccessTokenPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RenewAccessTokenRequest {
	#[serde(skip)]
	pub refresh_token: Uuid,
	pub login_id: Uuid,
}

impl ApiRequest for RenewAccessTokenRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = false;

	type RequestPath = RenewAccessTokenPath;
	type RequestQuery = Self;
	type RequestBody = ();
	type Response = RenewAccessTokenResponse;

	fn extra_headers(&self) -> HeaderMap {
		let mut map = HeaderMap::new();
		map.insert(
			header::AUTHORIZATION,
			HeaderValue::from_str(self.refresh_token.as_str())
				.expect("refresh_token to have valid ASCII characters"),
		);
		map
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RenewAccessTokenResponse {
	pub access_token: String,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{RenewAccessTokenRequest, RenewAccessTokenResponse};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&RenewAccessTokenRequest {
				login_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				refresh_token: Uuid::nil(),
			},
			&[
				Token::Struct {
					name: "RenewAccessTokenRequest",
					len: 1,
				},
				Token::Str("loginId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		const ACCESS_TOKEN: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJodHRwczovL2FwaS5wYXRyLmNsb3VkIiwiYXVkIjoiaHR0cHM6Ly8qLnBhdHIuY2xvdWQiLCJpYXQiOjE2MzUyMzM2NDg2MzAsInR5cCI6ImFjY2Vzc1Rva2VuIiwiZXhwIjoxNjM1NDkyODQ4NjMwLCJvcmdzIjp7IjlhNDY2MmE3NTIzZDQ3OTFiMzJlMTAxOTY2MjQ1Njc1Ijp7ImlzU3VwZXJBZG1pbiI6dHJ1ZSwicmVzb3VyY2VzIjp7fSwicmVzb3VyY2VUeXBlcyI6e319fSwibG9naW5JZCI6ImYwMjliMjE1OWEyNjQ2MjU4MmExNDJjYmMzMGU2NTEyIiwidXNlciI6eyJpZCI6WzExLDE3OSwxNzEsMzEsNDUsNjQsNzcsMTg1LDE1Myw4MCwyNTMsMjksMzgsMzEsMTU1LDE2NV0sInVzZXJuYW1lIjoicmFrc2hpdGgtcmF2aSIsImZpcnN0TmFtZSI6IlJha3NoaXRoIiwibGFzdE5hbWUiOiJSYXZpIiwiY3JlYXRlZCI6MTYzNDg5MTE0NTMxNX19.L_xFtH-gN8AjOVwSnz4ruh3gAUgr94DwML2pIdrwMzc";
		assert_tokens(
			&RenewAccessTokenResponse {
				access_token: ACCESS_TOKEN.to_string(),
			},
			&[
				Token::Struct {
					name: "RenewAccessTokenResponse",
					len: 1,
				},
				Token::Str("accessToken"),
				Token::Str(ACCESS_TOKEN),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		const ACCESS_TOKEN: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpc3MiOiJodHRwczovL2FwaS5wYXRyLmNsb3VkIiwiYXVkIjoiaHR0cHM6Ly8qLnBhdHIuY2xvdWQiLCJpYXQiOjE2MzUyMzM2NDg2MzAsInR5cCI6ImFjY2Vzc1Rva2VuIiwiZXhwIjoxNjM1NDkyODQ4NjMwLCJvcmdzIjp7IjlhNDY2MmE3NTIzZDQ3OTFiMzJlMTAxOTY2MjQ1Njc1Ijp7ImlzU3VwZXJBZG1pbiI6dHJ1ZSwicmVzb3VyY2VzIjp7fSwicmVzb3VyY2VUeXBlcyI6e319fSwibG9naW5JZCI6ImYwMjliMjE1OWEyNjQ2MjU4MmExNDJjYmMzMGU2NTEyIiwidXNlciI6eyJpZCI6WzExLDE3OSwxNzEsMzEsNDUsNjQsNzcsMTg1LDE1Myw4MCwyNTMsMjksMzgsMzEsMTU1LDE2NV0sInVzZXJuYW1lIjoicmFrc2hpdGgtcmF2aSIsImZpcnN0TmFtZSI6IlJha3NoaXRoIiwibGFzdE5hbWUiOiJSYXZpIiwiY3JlYXRlZCI6MTYzNDg5MTE0NTMxNX19.L_xFtH-gN8AjOVwSnz4ruh3gAUgr94DwML2pIdrwMzc";
		assert_tokens(
			&ApiResponse::success(RenewAccessTokenResponse {
				access_token: ACCESS_TOKEN.to_string(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("accessToken"),
				Token::Str(ACCESS_TOKEN),
				Token::MapEnd,
			],
		);
	}
}
