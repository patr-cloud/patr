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
#[typed_path(
	"/workspace/:workspace_id/infrastructure/static-site/:static_site_id/"
)]
pub struct UpdateStaticSitePath {
	pub workspace_id: Uuid,
	pub static_site_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStaticSiteRequest {
	pub name: String,
}

impl ApiRequest for UpdateStaticSiteRequest {
	const METHOD: Method = Method::PATCH;
	const IS_PROTECTED: bool = true;

	type RequestPath = UpdateStaticSitePath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::UpdateStaticSiteRequest;
	use crate::{ApiRequest, ApiResponse};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&UpdateStaticSiteRequest {
				name: "patr".to_string(),
			},
			&[
				Token::Struct {
					name: "UpdateStaticSiteRequest",
					len: 1,
				},
				Token::Str("name"),
				Token::Str("patr"),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<UpdateStaticSiteRequest as ApiRequest>::Response>(
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
