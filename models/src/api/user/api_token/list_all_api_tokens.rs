use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::UserApiToken;
use crate::{utils::Paginated, ApiRequest};

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
pub struct ListApiTokensPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListApiTokensRequest;

impl ApiRequest for ListApiTokensRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListApiTokensPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListApiTokenResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListApiTokenResponse {
	pub tokens: Vec<UserApiToken>,
}

#[cfg(test)]
mod test {
	use std::str::FromStr;

	use chrono::{TimeZone, Utc};
	use ipnetwork::IpNetwork;
	use serde_test::{assert_tokens, Token};

	use super::{ListApiTokenResponse, ListApiTokensRequest};
	use crate::{
		models::user::api_token::UserApiToken,
		utils::Uuid,
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListApiTokensRequest,
			&[Token::UnitStruct {
				name: "ListApiTokensRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListApiTokenResponse {
				tokens: vec![
					UserApiToken {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "Token 1".to_string(),
						token_nbf: None,
						token_exp: None,
						allowed_ips: None,
						created: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
					},
					UserApiToken {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "Token 2".to_string(),
						token_nbf: Some(
							Utc.timestamp_opt(1431648000, 0).unwrap().into(),
						),
						token_exp: Some(
							Utc.timestamp_opt(1431648000, 0).unwrap().into(),
						),
						allowed_ips: Some(vec![
							IpNetwork::from_str("1.1.1.1").unwrap(),
							IpNetwork::from_str("1.0.0.0/8").unwrap(),
						]),
						created: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
					},
				],
			},
			&[
				Token::Struct {
					name: "ListApiTokenResponse",
					len: 1,
				},
				Token::Str("tokens"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "UserApiToken",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("Token 1"),
				Token::Str("created"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::StructEnd,
				Token::Struct {
					name: "UserApiToken",
					len: 6,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("Token 2"),
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
				Token::Str("1.0.0.0/8"),
				Token::SeqEnd,
				Token::Str("created"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListApiTokenResponse {
				tokens: vec![
					UserApiToken {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "Token 1".to_string(),
						token_nbf: None,
						token_exp: None,
						allowed_ips: None,
						created: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
					},
					UserApiToken {
						id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
							.unwrap(),
						name: "Token 2".to_string(),
						token_nbf: Some(
							Utc.timestamp_opt(1431648000, 0).unwrap().into(),
						),
						token_exp: Some(
							Utc.timestamp_opt(1431648000, 0).unwrap().into(),
						),
						allowed_ips: Some(vec![
							IpNetwork::from_str("1.1.1.1").unwrap(),
							IpNetwork::from_str("1.0.0.0/8").unwrap(),
						]),
						created: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
					},
				],
			}),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("tokens"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "UserApiToken",
					len: 3,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("Token 1"),
				Token::Str("created"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::StructEnd,
				Token::Struct {
					name: "UserApiToken",
					len: 6,
				},
				Token::Str("id"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("name"),
				Token::Str("Token 2"),
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
				Token::Str("1.0.0.0/8"),
				Token::SeqEnd,
				Token::Str("created"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
