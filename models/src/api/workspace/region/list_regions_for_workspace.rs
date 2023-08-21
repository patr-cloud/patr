use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::Region;
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
#[typed_path("/workspace/:workspace_id/region")]
pub struct ListRegionsForWorkspacePath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListRegionsForWorkspaceRequest;

impl ApiRequest for ListRegionsForWorkspaceRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListRegionsForWorkspacePath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListRegionsForWorkspaceResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListRegionsForWorkspaceResponse {
	pub regions: Vec<Region>,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		ListRegionsForWorkspaceRequest,
		ListRegionsForWorkspaceResponse,
	};
	use crate::{
		models::workspace::region::{
			InfrastructureCloudProvider,
			Region,
			RegionStatus,
			RegionType,
		},
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListRegionsForWorkspaceRequest,
			&[Token::UnitStruct {
				name: "ListRegionsForWorkspaceRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListRegionsForWorkspaceResponse {
				regions: vec![
					Region {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "Bangalore".to_string(),
						cloud_provider:
							InfrastructureCloudProvider::Digitalocean,
						status: RegionStatus::Active,
						r#type: RegionType::PatrOwned,
					},
					Region {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "Bangalore".to_string(),
						cloud_provider:
							InfrastructureCloudProvider::Digitalocean,
						status: RegionStatus::Disconnected,
						r#type: RegionType::BYOC,
					},
				],
			},
			&[
				Token::Struct {
					name: "ListRegionsForWorkspaceResponse",
					len: 1,
				},
				Token::Str("regions"),
				Token::Seq { len: Some(2) },
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("Bangalore"),
				Token::Str("cloudProvider"),
				Token::UnitVariant {
					name: "InfrastructureCloudProvider",
					variant: "digitalocean",
				},
				Token::Str("status"),
				Token::UnitVariant {
					name: "RegionStatus",
					variant: "active",
				},
				Token::Str("type"),
				Token::Str("patrOwned"),
				Token::MapEnd,
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("Bangalore"),
				Token::Str("cloudProvider"),
				Token::UnitVariant {
					name: "InfrastructureCloudProvider",
					variant: "digitalocean",
				},
				Token::Str("status"),
				Token::UnitVariant {
					name: "RegionStatus",
					variant: "disconnected",
				},
				Token::Str("type"),
				Token::Str("byoc"),
				Token::MapEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListRegionsForWorkspaceResponse {
				regions: vec![
					Region {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "Bangalore".to_string(),
						cloud_provider:
							InfrastructureCloudProvider::Digitalocean,
						status: RegionStatus::Active,
						r#type: RegionType::PatrOwned,
					},
					Region {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "Bangalore".to_string(),
						cloud_provider:
							InfrastructureCloudProvider::Digitalocean,
						status: RegionStatus::Disconnected,
						r#type: RegionType::BYOC,
					},
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("regions"),
				Token::Seq { len: Some(2) },
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("Bangalore"),
				Token::Str("cloudProvider"),
				Token::UnitVariant {
					name: "InfrastructureCloudProvider",
					variant: "digitalocean",
				},
				Token::Str("status"),
				Token::UnitVariant {
					name: "RegionStatus",
					variant: "active",
				},
				Token::Str("type"),
				Token::Str("patrOwned"),
				Token::MapEnd,
				Token::Map { len: None },
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("Bangalore"),
				Token::Str("cloudProvider"),
				Token::UnitVariant {
					name: "InfrastructureCloudProvider",
					variant: "digitalocean",
				},
				Token::Str("status"),
				Token::UnitVariant {
					name: "RegionStatus",
					variant: "disconnected",
				},
				Token::Str("type"),
				Token::Str("byoc"),
				Token::MapEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
