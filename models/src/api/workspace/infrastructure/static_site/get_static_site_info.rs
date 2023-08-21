use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::{StaticSite, StaticSiteDetails};
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
pub struct GetStaticSiteInfoPath {
	pub workspace_id: Uuid,
	pub static_site_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetStaticSiteInfoRequest {}

impl ApiRequest for GetStaticSiteInfoRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetStaticSiteInfoPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetStaticSiteInfoResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetStaticSiteInfoResponse {
	#[serde(flatten)]
	pub static_site: StaticSite,
	#[serde(flatten)]
	pub static_site_details: StaticSiteDetails,
}

#[cfg(test)]
mod test {

	use serde_test::{assert_tokens, Token};

	use super::{GetStaticSiteInfoRequest, GetStaticSiteInfoResponse};
	use crate::{
		models::workspace::infrastructure::{
			deployment::DeploymentStatus,
			static_site::{StaticSite, StaticSiteDetails},
		},
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&GetStaticSiteInfoRequest {},
			&[
				Token::Struct {
					name: "GetStaticSiteInfoRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetStaticSiteInfoResponse {
				static_site: StaticSite {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "John Patr's static site".to_string(),
					status: DeploymentStatus::Running,
					current_live_upload: Some(
						Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
							.unwrap(),
					),
				},
				static_site_details: StaticSiteDetails {},
			},
			&[
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("John Patr's static site"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "DeploymentStatus",
					variant: "running",
				},
				Token::Str("currentLiveUpload"),
				Token::Some,
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::MapEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetStaticSiteInfoResponse {
				static_site: StaticSite {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "John Patr's static site".to_string(),
					status: DeploymentStatus::Running,
					current_live_upload: Some(
						Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
							.unwrap(),
					),
				},
				static_site_details: StaticSiteDetails {},
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("John Patr's static site"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "DeploymentStatus",
					variant: "running",
				},
				Token::Str("currentLiveUpload"),
				Token::Some,
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::MapEnd,
			],
		)
	}
}
