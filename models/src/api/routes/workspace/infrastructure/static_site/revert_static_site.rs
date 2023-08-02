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
#[typed_path("/workspace/:workspace_id/infrastructure/static-site/:static_site_id/upload/:upload_id/revert")]
pub struct RevertStaticSitePath {
	pub workspace_id: Uuid,
	pub static_site_id: Uuid,
	pub upload_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RevertStaticSiteRequest {}

impl ApiRequest for RevertStaticSiteRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = RevertStaticSitePath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::RevertStaticSiteRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&RevertStaticSiteRequest {},
			&[
				Token::Struct {
					name: "RevertStaticSiteRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<RevertStaticSiteRequest as ApiRequest>::Response>(
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
		)
	}
}
