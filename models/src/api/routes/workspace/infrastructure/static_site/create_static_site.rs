use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::StaticSiteDetails;
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
#[typed_path("/workspace/:workspace_id/infrastructure/static-site")]
pub struct CreateStaticSitePath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateStaticSiteRequest {
	pub name: String,
	pub message: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub file: Option<String>,
	#[serde(flatten)]
	pub static_site_details: StaticSiteDetails,
}

impl ApiRequest for CreateStaticSiteRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = CreateStaticSitePath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = CreateStaticSiteResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateStaticSiteResponse {
	pub id: Uuid,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{CreateStaticSiteRequest, CreateStaticSiteResponse};
	use crate::{
		models::workspace::infrastructure::static_site::StaticSiteDetails,
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&CreateStaticSiteRequest {
				name: "John Patr's static site".to_string(),
				message: "v1".to_string(),
				file: Some(
					"dGhpcyBpcyBhIGNvbXByZXNzZWQgc3RhdGljIHNpdGUK".to_string(),
				),
				static_site_details: StaticSiteDetails {},
			},
			&[
				Token::Map { len: None },
				Token::Str("name"),
				Token::Str("John Patr's static site"),
				Token::Str("message"),
				Token::Str("v1"),
				Token::Str("file"),
				Token::Some,
				Token::Str("dGhpcyBpcyBhIGNvbXByZXNzZWQgc3RhdGljIHNpdGUK"),
				Token::MapEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&CreateStaticSiteResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "CreateStaticSiteResponse",
					len: 1,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(CreateStaticSiteResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::MapEnd,
			],
		)
	}
}
