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
	Eq,
	Ord,
	Copy,
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
#[typed_path("/user/api-token")]
pub struct CreateApiTokenPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiTokenRequest {
	pub name: String,
	pub permissions: BTreeMap<Uuid, WorkspacePermission>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub token_nbf: Option<DateTime<Utc>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub token_exp: Option<DateTime<Utc>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub allowed_ips: Option<Vec<IpNetwork>>,
}

impl ApiRequest for CreateApiTokenRequest {
	const METHOD: Method = Method::POST;
	const IS_PROTECTED: bool = true;

	type RequestPath = CreateApiTokenPath;
	type RequestQuery = ();
	type RequestBody = Self;
	type Response = CreateApiTokenResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiTokenResponse {
	pub id: Uuid,
	pub token: String,
}

#[cfg(test)]
mod test {
	use std::{collections::BTreeMap, str::FromStr};

	use chrono::{TimeZone, Utc};
	use ipnetwork::IpNetwork;
	use serde_test::{assert_tokens, Token};

	use super::{CreateApiTokenRequest, CreateApiTokenResponse};
	use crate::{
		models::workspace::WorkspacePermission,
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&CreateApiTokenRequest {
				name: "my-first-token".to_string(),
				permissions: {
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
				},
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
					name: "CreateApiTokenRequest",
					len: 5,
				},
				Token::Str("name"),
				Token::Str("my-first-token"),
				Token::Str("permissions"),
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
		assert_tokens(
			&CreateApiTokenResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				token: String::from("token"),
			},
			&[
				Token::Struct {
					name: "CreateApiTokenResponse",
					len: 2,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("token"),
				Token::Str("token"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(CreateApiTokenResponse {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				token: String::from("token"),
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("token"),
				Token::Str("token"),
				Token::MapEnd,
			],
		);
	}
}
