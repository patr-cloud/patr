use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::StaticSite;
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
#[typed_path("/workspace/:workspace_id/infrastructure/static-site")]
pub struct ListStaticSitesPath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListStaticSitesRequest {}

impl ApiRequest for ListStaticSitesRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListStaticSitesPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListStaticSitesResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListStaticSitesResponse {
	pub static_sites: Vec<StaticSite>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{ListStaticSitesRequest, ListStaticSitesResponse};
	use crate::{
		models::workspace::infrastructure::{
			deployment::DeploymentStatus,
			static_site::StaticSite,
		},
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListStaticSitesRequest {},
			&[
				Token::Struct {
					name: "ListStaticSitesRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListStaticSitesResponse {
				static_sites: vec![
					StaticSite {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "John Patr's static site".to_string(),
						status: DeploymentStatus::Running,
						current_live_upload: None,
					},
					StaticSite {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
							.unwrap(),
						name: "John Patr's other static site".to_string(),
						status: DeploymentStatus::Stopped,
						current_live_upload: None,
					},
				],
			},
			&[
				Token::Struct {
					name: "ListStaticSitesResponse",
					len: 1,
				},
				Token::Str("staticSites"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "StaticSite",
					len: 4,
				},
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
				Token::None,
				Token::StructEnd,
				Token::Struct {
					name: "StaticSite",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("name"),
				Token::Str("John Patr's other static site"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "DeploymentStatus",
					variant: "stopped",
				},
				Token::Str("currentLiveUpload"),
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
			&ApiResponse::success(ListStaticSitesResponse {
				static_sites: vec![
					StaticSite {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "John Patr's static site".to_string(),
						status: DeploymentStatus::Running,
						current_live_upload: None,
					},
					StaticSite {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30868")
							.unwrap(),
						name: "John Patr's other static site".to_string(),
						status: DeploymentStatus::Deploying,
						current_live_upload: None,
					},
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("staticSites"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "StaticSite",
					len: 4,
				},
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
				Token::None,
				Token::StructEnd,
				Token::Struct {
					name: "StaticSite",
					len: 4,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("name"),
				Token::Str("John Patr's other static site"),
				Token::Str("status"),
				Token::UnitVariant {
					name: "DeploymentStatus",
					variant: "deploying",
				},
				Token::Str("currentLiveUpload"),
				Token::None,
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		)
	}
}
