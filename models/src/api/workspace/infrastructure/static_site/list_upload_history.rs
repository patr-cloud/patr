use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::StaticSiteUploadHistory;
use crate::{
	utils::{Paginated, Uuid},
	ApiRequest,
};

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
pub struct ListStaticSiteUploadHistoryPath {
	pub workspace_id: Uuid,
	pub static_site_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListStaticSiteUploadHistoryRequest {}

impl ApiRequest for ListStaticSiteUploadHistoryRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListStaticSiteUploadHistoryPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListStaticSiteUploadHistoryResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListStaticSiteUploadHistoryResponse {
	pub uploads: Vec<StaticSiteUploadHistory>,
}

#[cfg(test)]
mod test {
	use std::str::FromStr;

	use serde_test::{assert_tokens, Token};

	use super::{
		ListStaticSiteUploadHistoryRequest,
		ListStaticSiteUploadHistoryResponse,
	};
	use crate::{
		models::workspace::infrastructure::static_site::StaticSiteUploadHistory,
		utils::{DateTime, Uuid},
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListStaticSiteUploadHistoryRequest {},
			&[
				Token::Struct {
					name: "ListStaticSiteUploadHistoryRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListStaticSiteUploadHistoryResponse {
				uploads: vec![
					StaticSiteUploadHistory {
						upload_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						message: "v2".to_string(),
						uploaded_by: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						created: DateTime::from_str(
							"2020-04-12 22:10:57+02:00",
						)
						.unwrap(),
						processed: Some(
							DateTime::from_str("2020-04-12 22:10:57+02:00")
								.unwrap(),
						),
					},
					StaticSiteUploadHistory {
						upload_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30868",
						)
						.unwrap(),
						message: "v3".to_string(),
						uploaded_by: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						created: DateTime::from_str(
							"2020-04-12 22:10:57+02:00",
						)
						.unwrap(),
						processed: None,
					},
				],
			},
			&[
				Token::Struct {
					name: "ListStaticSiteUploadHistoryResponse",
					len: 1,
				},
				Token::Str("uploads"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "StaticSiteUploadHistory",
					len: 5,
				},
				Token::Str("uploadId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("message"),
				Token::Str("v2"),
				Token::Str("uploadedBy"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("created"),
				Token::Str("Sun, 12 Apr 2020 20:10:57 +0000"),
				Token::Str("processed"),
				Token::Some,
				Token::Str("Sun, 12 Apr 2020 20:10:57 +0000"),
				Token::StructEnd,
				Token::Struct {
					name: "StaticSiteUploadHistory",
					len: 5,
				},
				Token::Str("uploadId"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("message"),
				Token::Str("v3"),
				Token::Str("uploadedBy"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("created"),
				Token::Str("Sun, 12 Apr 2020 20:10:57 +0000"),
				Token::Str("processed"),
				Token::None,
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListStaticSiteUploadHistoryResponse {
				uploads: vec![
					StaticSiteUploadHistory {
						upload_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						message: "v2".to_string(),
						uploaded_by: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						created: DateTime::from_str(
							"2020-04-12 22:10:57+02:00",
						)
						.unwrap(),
						processed: Some(
							DateTime::from_str("2020-04-12 22:10:57+02:00")
								.unwrap(),
						),
					},
					StaticSiteUploadHistory {
						upload_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30868",
						)
						.unwrap(),
						message: "v3".to_string(),
						uploaded_by: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						created: DateTime::from_str(
							"2020-04-12 22:10:57+02:00",
						)
						.unwrap(),
						processed: None,
					},
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("uploads"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "StaticSiteUploadHistory",
					len: 5,
				},
				Token::Str("uploadId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("message"),
				Token::Str("v2"),
				Token::Str("uploadedBy"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("created"),
				Token::Str("Sun, 12 Apr 2020 20:10:57 +0000"),
				Token::Str("processed"),
				Token::Some,
				Token::Str("Sun, 12 Apr 2020 20:10:57 +0000"),
				Token::StructEnd,
				Token::Struct {
					name: "StaticSiteUploadHistory",
					len: 5,
				},
				Token::Str("uploadId"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("message"),
				Token::Str("v3"),
				Token::Str("uploadedBy"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("created"),
				Token::Str("Sun, 12 Apr 2020 20:10:57 +0000"),
				Token::Str("processed"),
				Token::None,
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
