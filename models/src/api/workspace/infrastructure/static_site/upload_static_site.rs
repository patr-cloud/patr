use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

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
#[typed_path("/workspace/:workspace_id/infrastructure/static-site/:static_site_id/upload")]
pub struct UploadStaticSitePath {
	pub workspace_id: Uuid,
	pub static_site_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UploadStaticSiteRequest {
	pub file: String,
	pub message: String,
}

impl ApiRequest for UploadStaticSiteRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = UploadStaticSitePath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = UploadStaticSiteResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UploadStaticSiteResponse {
	pub upload_id: Uuid,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{UploadStaticSiteRequest, UploadStaticSiteResponse};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&UploadStaticSiteRequest {
				file: "aGVsbG8gd29ybGQK".to_string(),
				message: "v1".to_string(),
			},
			&[
				Token::Struct {
					name: "UploadStaticSiteRequest",
					len: 2,
				},
				Token::Str("file"),
				Token::Str("aGVsbG8gd29ybGQK"),
				Token::Str("message"),
				Token::Str("v1"),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&UploadStaticSiteResponse {
				upload_id: Uuid::parse_str("35d20c5c66904bec84cc6e2a87a23265")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "UploadStaticSiteResponse",
					len: 1,
				},
				Token::Str("uploadId"),
				Token::Str("35d20c5c66904bec84cc6e2a87a23265"),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(UploadStaticSiteResponse {
				upload_id: Uuid::parse_str("35d20c5c66904bec84cc6e2a87a23265")
					.unwrap(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("uploadId"),
				Token::Str("35d20c5c66904bec84cc6e2a87a23265"),
				Token::MapEnd,
			],
		)
	}
}
