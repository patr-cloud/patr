use axum_extra::routing::TypedPath;
use chrono::Utc;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::Region;
use crate::{
	utils::{DateTime, Uuid},
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
#[typed_path("/workspace/:workspace_id/region/:region_id")]
pub struct GetRegionInfoPath {
	pub workspace_id: Uuid,
	pub region_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetRegionInfoRequest {}

impl ApiRequest for GetRegionInfoRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = GetRegionInfoPath;
	type RequestQuery = ();
	type RequestBody = ();
	type Response = GetRegionInfoResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetRegionInfoResponse {
	#[serde(flatten)]
	pub region: Region,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub message_log: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub disconnected_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod test {
	use chrono::{TimeZone, Utc};
	use serde_test::{assert_tokens, Token};

	use super::{GetRegionInfoRequest, GetRegionInfoResponse};
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
			&GetRegionInfoRequest {},
			&[
				Token::Struct {
					name: "GetRegionInfoRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&GetRegionInfoResponse {
				region: Region {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "Bangalore".to_string(),
					cloud_provider: InfrastructureCloudProvider::Digitalocean,
					status: RegionStatus::Active,
					r#type: RegionType::PatrOwned,
				},
				message_log: None,
				disconnected_at: None,
			},
			&[
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
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(GetRegionInfoResponse {
				region: Region {
					id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
						.unwrap(),
					name: "Bangalore".to_string(),
					cloud_provider: InfrastructureCloudProvider::Digitalocean,
					status: RegionStatus::Disconnected,
					r#type: RegionType::BYOC,
				},
				message_log: Some("Unknown error".into()),
				disconnected_at: Some(Utc.timestamp_opt(0, 0).unwrap().into()),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
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
				Token::Str("messageLog"),
				Token::Some,
				Token::Str("Unknown error"),
				Token::Str("disconnectedAt"),
				Token::Some,
				Token::Str("Thu, 01 Jan 1970 00:00:00 +0000"),
				Token::MapEnd,
			],
		);
	}
}
