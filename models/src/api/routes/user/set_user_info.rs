use axum_extra::routing::TypedPath;
use chrono::Utc;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{utils::DateTime, ApiRequest};

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
#[typed_path("/user/info")]
pub struct UpdateUserInfoPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserInfoRequest {
	pub first_name: Option<String>,
	pub last_name: Option<String>,
	pub birthday: Option<DateTime<Utc>>,
	pub bio: Option<String>,
	pub location: Option<String>,
}

impl ApiRequest for UpdateUserInfoRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = UpdateUserInfoPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use chrono::{TimeZone, Utc};
	use serde_test::{assert_tokens, Token};

	use super::UpdateUserInfoRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&UpdateUserInfoRequest {
				first_name: Some(String::from("John")),
				last_name: Some(String::from("Patr")),
				birthday: Some(Utc.timestamp_opt(0, 0).unwrap().into()),
				bio: Some(String::from("I'm a random bot")),
				location: Some(String::from("Somewhere in the Internet")),
			},
			&[
				Token::Struct {
					name: "UpdateUserInfoRequest",
					len: 5,
				},
				Token::Str("firstName"),
				Token::Some,
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Some,
				Token::Str("Patr"),
				Token::Str("birthday"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::Str("bio"),
				Token::Some,
				Token::Str("I'm a random bot"),
				Token::Str("location"),
				Token::Some,
				Token::Str("Somewhere in the Internet"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<UpdateUserInfoRequest as ApiRequest>::Response>(
			(),
		);
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
