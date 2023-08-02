use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::BasicUserInfo;
use crate::{utils::Paginated, ApiRequest};

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
#[typed_path("/user/search")]
pub struct GetVersionPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchForUserRequest {
	pub query: String,
}

impl ApiRequest for SearchForUserRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetVersionPath;
	type RequestQuery = Paginated<Self>;
	type RequestBody = ();
	type Response = SearchForUserResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchForUserResponse {
	pub users: Vec<BasicUserInfo>,
}

#[cfg(test)]
mod tests {
	use serde_test::{assert_tokens, Token};

	use super::{SearchForUserRequest, SearchForUserResponse};
	use crate::{models::user::BasicUserInfo, utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&SearchForUserRequest {
				query: "query string".to_string(),
			},
			&[
				Token::Struct {
					name: "SearchForUserRequest",
					len: 1,
				},
				Token::Str("query"),
				Token::Str("query string"),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&SearchForUserResponse {
				users: vec![
					// without bio, location
					BasicUserInfo {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						username: "john-patr".to_string(),
						first_name: "John".to_string(),
						last_name: "Patr".to_string(),
						bio: None,
						location: None,
					},
					// with bio, without location
					BasicUserInfo {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						username: "john-patr".to_string(),
						first_name: "John".to_string(),
						last_name: "Patr".to_string(),
						bio: Some("I'm a random bot".to_string()),
						location: None,
					},
					// with location, without bio
					BasicUserInfo {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						username: "john-patr".to_string(),
						first_name: "John".to_string(),
						last_name: "Patr".to_string(),
						bio: None,
						location: Some("Somewhere in the internet".to_string()),
					},
					// with bio, location
					BasicUserInfo {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						username: "john-patr".to_string(),
						first_name: "John".to_string(),
						last_name: "Patr".to_string(),
						bio: Some("I'm a random bot".to_string()),
						location: Some("Somewhere in the internet".to_string()),
					},
				],
			},
			&[
				Token::Struct {
					name: "SearchForUserResponse",
					len: 1,
				},
				Token::Str("users"),
				Token::Seq { len: Some(4) },
				Token::Struct {
					name: "BasicUserInfo",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::StructEnd,
				Token::Struct {
					name: "BasicUserInfo",
					len: 5,
				},
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
				Token::StructEnd,
				Token::Struct {
					name: "BasicUserInfo",
					len: 5,
				},
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
				Token::StructEnd,
				Token::Struct {
					name: "BasicUserInfo",
					len: 6,
				},
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
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(SearchForUserResponse {
				users: vec![
					// without bio, location
					BasicUserInfo {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						username: "john-patr".to_string(),
						first_name: "John".to_string(),
						last_name: "Patr".to_string(),
						bio: None,
						location: None,
					},
					// with bio, without location
					BasicUserInfo {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						username: "john-patr".to_string(),
						first_name: "John".to_string(),
						last_name: "Patr".to_string(),
						bio: Some("I'm a random bot".to_string()),
						location: None,
					},
					// with location, without bio
					BasicUserInfo {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						username: "john-patr".to_string(),
						first_name: "John".to_string(),
						last_name: "Patr".to_string(),
						bio: None,
						location: Some("Somewhere in the internet".to_string()),
					},
					// with bio, location
					BasicUserInfo {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						username: "john-patr".to_string(),
						first_name: "John".to_string(),
						last_name: "Patr".to_string(),
						bio: Some("I'm a random bot".to_string()),
						location: Some("Somewhere in the internet".to_string()),
					},
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("users"),
				Token::Seq { len: Some(4) },
				Token::Struct {
					name: "BasicUserInfo",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("username"),
				Token::Str("john-patr"),
				Token::Str("firstName"),
				Token::Str("John"),
				Token::Str("lastName"),
				Token::Str("Patr"),
				Token::StructEnd,
				Token::Struct {
					name: "BasicUserInfo",
					len: 5,
				},
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
				Token::StructEnd,
				Token::Struct {
					name: "BasicUserInfo",
					len: 5,
				},
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
				Token::StructEnd,
				Token::Struct {
					name: "BasicUserInfo",
					len: 6,
				},
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
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
