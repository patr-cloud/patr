use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::BasicUserInfo;
use crate::{prelude::Uuid, ApiRequest};

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
#[typed_path("/user/:user_id/info")]
pub struct ChangePasswordPath {
	pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetUserInfoByUserIdRequest {
	pub user_id: Uuid,
}

impl ApiRequest for GetUserInfoByUserIdRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ChangePasswordPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = GetUserInfoByUserIdResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetUserInfoByUserIdResponse {
	#[serde(flatten)]
	pub basic_user_info: BasicUserInfo,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{GetUserInfoByUserIdRequest, GetUserInfoByUserIdResponse};
	use crate::{models::user::BasicUserInfo, utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetUserInfoByUserIdRequest {
				user_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "GetUserInfoByUserIdRequest",
					len: 1,
				},
				Token::Str("userId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetUserInfoByUserIdResponse {
				basic_user_info: BasicUserInfo {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					username: "john-patr".to_string(),
					first_name: "John".to_string(),
					last_name: "Patr".to_string(),
					bio: None,
					location: None,
				},
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_with_bio() {
		assert_tokens(
			&ApiResponse::success(GetUserInfoByUserIdResponse {
				basic_user_info: BasicUserInfo {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					username: "john-patr".to_string(),
					first_name: "John".to_string(),
					last_name: "Patr".to_string(),
					bio: Some("I'm a random bot".to_string()),
					location: None,
				},
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("bio"),
				Token::Some,
				Token::Str("I'm a random bot"),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_with_location() {
		assert_tokens(
			&ApiResponse::success(GetUserInfoByUserIdResponse {
				basic_user_info: BasicUserInfo {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					username: "john-patr".to_string(),
					first_name: "John".to_string(),
					last_name: "Patr".to_string(),
					bio: None,
					location: Some("Somewhere in the internet".to_string()),
				},
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("location"),
				Token::Some,
				Token::Str("Somewhere in the internet"),
				Token::MapEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types_with_bio_and_location() {
		assert_tokens(
			&ApiResponse::success(GetUserInfoByUserIdResponse {
				basic_user_info: BasicUserInfo {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					username: "john-patr".to_string(),
					first_name: "John".to_string(),
					last_name: "Patr".to_string(),
					bio: Some("I'm a random bot".to_string()),
					location: Some("Somewhere in the internet".to_string()),
				},
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::Str("bio"),
				Token::Some,
				Token::Str("I'm a random bot"),
				Token::Str("location"),
				Token::Some,
				Token::Str("Somewhere in the internet"),
				Token::MapEnd,
			],
		);
	}
}
