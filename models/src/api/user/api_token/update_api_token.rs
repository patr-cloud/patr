use std::collections::BTreeMap;

use axum_extra::routing::TypedPath;
use chrono::Utc;
use ipnetwork::IpNetwork;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{
	models::workspace::WorkspacePermission,
	utils::{DateTime, Uuid},
	ApiRequest,
};

#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Default,
	TypedPath,
	Serialize,
	Deserialize,
)]
#[typed_path("/user/api-token/:token_id")]
pub struct UpdateApiTokenPath {
	pub token_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApiTokenRequest {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub permissions: Option<BTreeMap<Uuid, WorkspacePermission>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub token_nbf: Option<DateTime<Utc>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub token_exp: Option<DateTime<Utc>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub allowed_ips: Option<Vec<IpNetwork>>,
}

impl ApiRequest for UpdateApiTokenRequest {
	const METHOD: Method = Method::PATCH;
	const IS_PROTECTED: bool = true;

	type RequestPath = UpdateApiTokenPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = ();
}

#[cfg(test)]
mod test {
	use std::{collections::BTreeMap, str::FromStr};

	use chrono::{TimeZone, Utc};
	use ipnetwork::IpNetwork;
	use serde_test::{assert_tokens, Token};

	use super::UpdateApiTokenRequest;
	use crate::{
		models::workspace::WorkspacePermission,
		utils::Uuid,
		ApiRequest,
		ApiResponse,
	};

	#[test]
	fn assert_empty_request_types() {
		assert_tokens(
			&UpdateApiTokenRequest {
				name: None,
				permissions: None,
				token_nbf: None,
				token_exp: None,
				allowed_ips: None,
			},
			&[
				Token::Struct {
					name: "UpdateApiTokenRequest",
					len: 0,
				},
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_filled_request_types() {
		assert_tokens(
			&UpdateApiTokenRequest {
				name: Some("my-first-token".to_string()),
				permissions: Some({
					let mut map = BTreeMap::new();

					map.insert(
						Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						WorkspacePermission {
							is_super_admin: true,
							resource_permissions: BTreeMap::new(),
							resource_type_permissions: BTreeMap::new(),
						},
					);

					map
				}),
				token_nbf: Some(
					Utc.timestamp_opt(1431648000, 0).unwrap().into(),
				),
				token_exp: Some(
					Utc.timestamp_opt(1431648000, 0).unwrap().into(),
				),
				allowed_ips: Some(vec![
					IpNetwork::from_str("1.1.1.1").unwrap(),
					IpNetwork::from_str("1.0.0.1").unwrap(),
				]),
			},
			&[
				Token::Struct {
					name: "UpdateApiTokenRequest",
					len: 5,
				},
				Token::Str("name"),
				Token::Some,
				Token::Str("my-first-token"),
				Token::Str("permissions"),
				Token::Some,
				Token::Map { len: Some(1) },
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Struct {
					name: "WorkspacePermission",
					len: 3,
				},
				Token::Str("isSuperAdmin"),
				Token::Bool(true),
				Token::Str("resourcePermissions"),
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::Str("resourceTypePermissions"),
				Token::Map { len: Some(0) },
				Token::MapEnd,
				Token::StructEnd,
				Token::MapEnd,
				Token::Str("tokenNbf"),
				Token::Some,
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("tokenExp"),
				Token::Some,
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("allowedIps"),
				Token::Some,
				Token::Seq { len: Some(2) },
				Token::Str("1.1.1.1/32"),
				Token::Str("1.0.0.1/32"),
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_response_types() {
		crate::assert_types::<<UpdateApiTokenRequest as ApiRequest>::Response>(
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
		);
	}
}
