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
#[typed_path("/auth/sign-out")]
pub struct LogoutPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LogoutRequest;

impl ApiRequest for LogoutRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = false;

	type RequestPath = LogoutPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::LogoutRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&LogoutRequest,
			&[Token::UnitStruct {
				name: "LogoutRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<LogoutRequest as ApiRequest>::Response>(());
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
