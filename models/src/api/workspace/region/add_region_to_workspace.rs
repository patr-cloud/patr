use std::fmt;

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
#[typed_path("/workspace/:workspace_id/region")]
pub struct AddRegionToWorkspacePath {
	pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DigitaloceanRegion {
	AMS2,
	AMS3,
	BLR1,
	FRA1,
	LON1,
	NYC1,
	NYC2,
	NYC3,
	SFO1,
	SFO2,
	SFO3,
	SGP1,
	TOR1,
}

impl fmt::Display for DigitaloceanRegion {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			DigitaloceanRegion::AMS2 => write!(f, "ams2"),
			DigitaloceanRegion::AMS3 => write!(f, "ams3"),
			DigitaloceanRegion::BLR1 => write!(f, "blr1"),
			DigitaloceanRegion::FRA1 => write!(f, "fra1"),
			DigitaloceanRegion::LON1 => write!(f, "lon1"),
			DigitaloceanRegion::NYC1 => write!(f, "nyc1"),
			DigitaloceanRegion::NYC2 => write!(f, "nyc2"),
			DigitaloceanRegion::NYC3 => write!(f, "nyc3"),
			DigitaloceanRegion::SFO1 => write!(f, "sfo1"),
			DigitaloceanRegion::SFO2 => write!(f, "sfo2"),
			DigitaloceanRegion::SFO3 => write!(f, "sfo3"),
			DigitaloceanRegion::SGP1 => write!(f, "sgp1"),
			DigitaloceanRegion::TOR1 => write!(f, "tor1"),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AddRegionToWorkspaceData {
	#[serde(rename_all = "camelCase")]
	Digitalocean {
		cluster_name: String,
		region: DigitaloceanRegion,
		api_token: String,
		min_nodes: u16,
		max_nodes: u16,
		auto_scale: bool,
		node_name: String,
		node_size_slug: String,
	},
	#[serde(rename_all = "camelCase")]
	KubeConfig {
		config_file: kube::config::Kubeconfig,
	},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddRegionToWorkspaceRequest {
	pub name: String,
	#[serde(flatten)]
	pub data: AddRegionToWorkspaceData,
}

impl ApiRequest for AddRegionToWorkspaceRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = AddRegionToWorkspacePath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = AddRegionToWorkspaceResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddRegionToWorkspaceResponse {
	pub region_id: Uuid,
}

#[cfg(test)]
mod test {
	use serde_test::{assert_tokens, Token};

	use super::{
		AddRegionToWorkspaceData,
		AddRegionToWorkspaceRequest,
		AddRegionToWorkspaceResponse,
		DigitaloceanRegion,
	};
	use crate::{utils::Uuid, ApiResponse};

	#[test]
	fn assert_digitalocean_region_values() {
		for (region, value) in [
			(DigitaloceanRegion::AMS2, "ams2"),
			(DigitaloceanRegion::AMS3, "ams3"),
			(DigitaloceanRegion::BLR1, "blr1"),
			(DigitaloceanRegion::FRA1, "fra1"),
			(DigitaloceanRegion::LON1, "lon1"),
			(DigitaloceanRegion::NYC1, "nyc1"),
			(DigitaloceanRegion::NYC2, "nyc2"),
			(DigitaloceanRegion::NYC3, "nyc3"),
			(DigitaloceanRegion::SFO1, "sfo1"),
			(DigitaloceanRegion::SFO2, "sfo2"),
			(DigitaloceanRegion::SFO3, "sfo3"),
			(DigitaloceanRegion::SGP1, "sgp1"),
			(DigitaloceanRegion::TOR1, "tor1"),
		] {
			assert_tokens(
				&region,
				&[Token::UnitVariant {
					name: "DigitaloceanRegion",
					variant: value,
				}],
			)
		}
	}

	#[test]
	fn assert_add_region_to_workspace_data_digitalocean() {
		assert_tokens(
			&serde_json::to_value(AddRegionToWorkspaceData::Digitalocean {
				cluster_name: "patr-cluster".to_string(),
				region: DigitaloceanRegion::BLR1,
				api_token: "<api-token>".to_string(),
				min_nodes: 3,
				max_nodes: 5,
				auto_scale: true,
				node_name: "patr-node".to_string(),
				node_size_slug: "s-1vcpu-2gb".to_string(),
			})
			.unwrap(),
			&[
				Token::Map { len: Some(8) },
				Token::Str("clusterName"),
				Token::Str("patr-cluster"),
				Token::Str("region"),
				Token::Str("blr1"),
				Token::Str("apiToken"),
				Token::Str("<api-token>"),
				Token::Str("minNodes"),
				Token::U64(3),
				Token::Str("maxNodes"),
				Token::U64(5),
				Token::Str("autoScale"),
				Token::Bool(true),
				Token::Str("nodeName"),
				Token::Str("patr-node"),
				Token::Str("nodeSizeSlug"),
				Token::Str("s-1vcpu-2gb"),
				Token::MapEnd,
			],
		)
	}

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&serde_json::to_value(AddRegionToWorkspaceRequest {
				name: "Region Name".to_string(),
				data: AddRegionToWorkspaceData::Digitalocean {
					cluster_name: "patr-cluster".to_string(),
					region: DigitaloceanRegion::BLR1,
					api_token: "<api-token>".to_string(),
					min_nodes: 3,
					max_nodes: 5,
					auto_scale: true,
					node_name: "patr-node".to_string(),
					node_size_slug: "s-1vcpu-2gb".to_string(),
				},
			})
			.unwrap(),
			&[
				Token::Map { len: Some(9) },
				Token::Str("name"),
				Token::Str("Region Name"),
				Token::Str("clusterName"),
				Token::Str("patr-cluster"),
				Token::Str("region"),
				Token::Str("blr1"),
				Token::Str("apiToken"),
				Token::Str("<api-token>"),
				Token::Str("minNodes"),
				Token::U64(3),
				Token::Str("maxNodes"),
				Token::U64(5),
				Token::Str("autoScale"),
				Token::Bool(true),
				Token::Str("nodeName"),
				Token::Str("patr-node"),
				Token::Str("nodeSizeSlug"),
				Token::Str("s-1vcpu-2gb"),
				Token::MapEnd,
			],
		)
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&AddRegionToWorkspaceResponse {
				region_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			},
			&[
				Token::Struct {
					name: "AddRegionToWorkspaceResponse",
					len: 1,
				},
				Token::Str("regionId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::StructEnd,
			],
		)
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(AddRegionToWorkspaceResponse {
				region_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("regionId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::MapEnd,
			],
		)
	}
}
