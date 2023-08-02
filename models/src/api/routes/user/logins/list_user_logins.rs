use axum_extra::routing::TypedPath;
use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::UserWebLogin;
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
#[typed_path("/user/logins")]
pub struct ListUserLoginsPath;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListUserLoginsRequest;

impl ApiRequest for ListUserLoginsRequest {
	const METHOD: Method = Method::GET;
	const IS_PROTECTED: bool = true;

	type RequestPath = ListUserLoginsPath;
	type RequestQuery = Paginated;
	type RequestBody = ();
	type Response = ListUserLoginsResponse;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ListUserLoginsResponse {
	pub logins: Vec<UserWebLogin>,
}

#[cfg(test)]
mod test {
	use std::net::{IpAddr, Ipv4Addr};

	use chrono::{TimeZone, Utc};
	use serde_test::{assert_tokens, Configure, Token};

	use super::{ListUserLoginsRequest, ListUserLoginsResponse};
	use crate::{
		models::user::UserWebLogin,
		utils::{Location, Uuid},
		ApiResponse,
	};

	#[test]
	fn assert_request_types() {
		assert_tokens(
			&ListUserLoginsRequest,
			&[Token::UnitStruct {
				name: "ListUserLoginsRequest",
			}],
		);
	}

	#[test]
	fn assert_response_types() {
		assert_tokens(
			&ListUserLoginsResponse {
				logins: vec![
					UserWebLogin {
						login_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						token_expiry: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						created: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						created_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
						created_location: Location { lat: 0.0, lng: 0.0 },
						created_country: "IN".to_string(),
						created_region: "Karnataka".to_string(),
						created_city: "Bengaluru".to_string(),
						created_timezone: "UTC".to_string(),
						last_login: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						last_activity: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						last_activity_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
						last_activity_location: Location { lat: 0.0, lng: 0.0 },
						last_activity_user_agent: "user-agent".to_string(),
						last_activity_country: "IN".to_string(),
						last_activity_region: "Karnataka".to_string(),
						last_activity_city: "Bengaluru".to_string(),
						last_activity_timezone: "UTC".to_string(),
					},
					UserWebLogin {
						login_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30868",
						)
						.unwrap(),
						token_expiry: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						created: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						created_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
						created_location: Location { lat: 0.0, lng: 0.0 },
						created_country: "IN".to_string(),
						created_region: "Karnataka".to_string(),
						created_city: "Bengaluru".to_string(),
						created_timezone: "UTC".to_string(),
						last_login: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						last_activity: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						last_activity_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
						last_activity_location: Location { lat: 0.0, lng: 0.0 },
						last_activity_user_agent: "user-agent".to_string(),
						last_activity_country: "IN".to_string(),
						last_activity_region: "Karnataka".to_string(),
						last_activity_city: "Bengaluru".to_string(),
						last_activity_timezone: "UTC".to_string(),
					},
				],
			}
			.readable(),
			&[
				Token::Struct {
					name: "ListUserLoginsResponse",
					len: 1,
				},
				Token::Str("logins"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "UserWebLogin",
					len: 18,
				},
				Token::Str("loginId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("tokenExpiry"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("created"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("createdIp"),
				Token::Str("127.0.0.1"),
				Token::Str("createdLocation"),
				Token::Struct {
					name: "Location",
					len: 2,
				},
				Token::Str("lat"),
				Token::F64(0.0),
				Token::Str("lng"),
				Token::F64(0.0),
				Token::StructEnd,
				Token::Str("createdCountry"),
				Token::Str("IN"),
				Token::Str("createdRegion"),
				Token::Str("Karnataka"),
				Token::Str("createdCity"),
				Token::Str("Bengaluru"),
				Token::Str("createdTimezone"),
				Token::Str("UTC"),
				Token::Str("lastLogin"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("lastActivity"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("lastActivityIp"),
				Token::Str("127.0.0.1"),
				Token::Str("lastActivityLocation"),
				Token::Struct {
					name: "Location",
					len: 2,
				},
				Token::Str("lat"),
				Token::F64(0.0),
				Token::Str("lng"),
				Token::F64(0.0),
				Token::StructEnd,
				Token::Str("lastActivityUserAgent"),
				Token::Str("user-agent"),
				Token::Str("lastActivityCountry"),
				Token::Str("IN"),
				Token::Str("lastActivityRegion"),
				Token::Str("Karnataka"),
				Token::Str("lastActivityCity"),
				Token::Str("Bengaluru"),
				Token::Str("lastActivityTimezone"),
				Token::Str("UTC"),
				Token::StructEnd,
				Token::Struct {
					name: "UserWebLogin",
					len: 18,
				},
				Token::Str("loginId"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("tokenExpiry"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("created"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("createdIp"),
				Token::Str("127.0.0.1"),
				Token::Str("createdLocation"),
				Token::Struct {
					name: "Location",
					len: 2,
				},
				Token::Str("lat"),
				Token::F64(0.0),
				Token::Str("lng"),
				Token::F64(0.0),
				Token::StructEnd,
				Token::Str("createdCountry"),
				Token::Str("IN"),
				Token::Str("createdRegion"),
				Token::Str("Karnataka"),
				Token::Str("createdCity"),
				Token::Str("Bengaluru"),
				Token::Str("createdTimezone"),
				Token::Str("UTC"),
				Token::Str("lastLogin"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("lastActivity"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("lastActivityIp"),
				Token::Str("127.0.0.1"),
				Token::Str("lastActivityLocation"),
				Token::Struct {
					name: "Location",
					len: 2,
				},
				Token::Str("lat"),
				Token::F64(0.0),
				Token::Str("lng"),
				Token::F64(0.0),
				Token::StructEnd,
				Token::Str("lastActivityUserAgent"),
				Token::Str("user-agent"),
				Token::Str("lastActivityCountry"),
				Token::Str("IN"),
				Token::Str("lastActivityRegion"),
				Token::Str("Karnataka"),
				Token::Str("lastActivityCity"),
				Token::Str("Bengaluru"),
				Token::Str("lastActivityTimezone"),
				Token::Str("UTC"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_success_response_types() {
		assert_tokens(
			&ApiResponse::success(ListUserLoginsResponse {
				logins: vec![
					UserWebLogin {
						login_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30867",
						)
						.unwrap(),
						token_expiry: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						created: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						created_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
						created_location: Location { lat: 0.0, lng: 0.0 },
						created_country: "IN".to_string(),
						created_region: "Karnataka".to_string(),
						created_city: "Bengaluru".to_string(),
						created_timezone: "UTC".to_string(),
						last_login: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						last_activity: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						last_activity_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
						last_activity_location: Location { lat: 0.0, lng: 0.0 },
						last_activity_user_agent: "user-agent".to_string(),
						last_activity_country: "IN".to_string(),
						last_activity_region: "Karnataka".to_string(),
						last_activity_city: "Bengaluru".to_string(),
						last_activity_timezone: "UTC".to_string(),
					},
					UserWebLogin {
						login_id: Uuid::parse_str(
							"2aef18631ded45eb9170dc2166b30868",
						)
						.unwrap(),
						token_expiry: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						created: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						created_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
						created_location: Location { lat: 0.0, lng: 0.0 },
						created_country: "IN".to_string(),
						created_region: "Karnataka".to_string(),
						created_city: "Bengaluru".to_string(),
						created_timezone: "UTC".to_string(),
						last_login: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						last_activity: Utc
							.timestamp_opt(1431648000, 0)
							.unwrap()
							.into(),
						last_activity_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
						last_activity_location: Location { lat: 0.0, lng: 0.0 },
						last_activity_user_agent: "user-agent".to_string(),
						last_activity_country: "IN".to_string(),
						last_activity_region: "Karnataka".to_string(),
						last_activity_city: "Bengaluru".to_string(),
						last_activity_timezone: "UTC".to_string(),
					},
				],
			})
			.readable(),
			&[
				Token::Map { len: None },
				Token::Str("success"),
				Token::Bool(true),
				Token::Str("logins"),
				Token::Seq { len: Some(2) },
				Token::Struct {
					name: "UserWebLogin",
					len: 18,
				},
				Token::Str("loginId"),
				Token::Str("2aef18631ded45eb9170dc2166b30867"),
				Token::Str("tokenExpiry"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("created"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("createdIp"),
				Token::Str("127.0.0.1"),
				Token::Str("createdLocation"),
				Token::Struct {
					name: "Location",
					len: 2,
				},
				Token::Str("lat"),
				Token::F64(0.0),
				Token::Str("lng"),
				Token::F64(0.0),
				Token::StructEnd,
				Token::Str("createdCountry"),
				Token::Str("IN"),
				Token::Str("createdRegion"),
				Token::Str("Karnataka"),
				Token::Str("createdCity"),
				Token::Str("Bengaluru"),
				Token::Str("createdTimezone"),
				Token::Str("UTC"),
				Token::Str("lastLogin"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("lastActivity"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("lastActivityIp"),
				Token::Str("127.0.0.1"),
				Token::Str("lastActivityLocation"),
				Token::Struct {
					name: "Location",
					len: 2,
				},
				Token::Str("lat"),
				Token::F64(0.0),
				Token::Str("lng"),
				Token::F64(0.0),
				Token::StructEnd,
				Token::Str("lastActivityUserAgent"),
				Token::Str("user-agent"),
				Token::Str("lastActivityCountry"),
				Token::Str("IN"),
				Token::Str("lastActivityRegion"),
				Token::Str("Karnataka"),
				Token::Str("lastActivityCity"),
				Token::Str("Bengaluru"),
				Token::Str("lastActivityTimezone"),
				Token::Str("UTC"),
				Token::StructEnd,
				Token::Struct {
					name: "UserWebLogin",
					len: 18,
				},
				Token::Str("loginId"),
				Token::Str("2aef18631ded45eb9170dc2166b30868"),
				Token::Str("tokenExpiry"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("created"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("createdIp"),
				Token::Str("127.0.0.1"),
				Token::Str("createdLocation"),
				Token::Struct {
					name: "Location",
					len: 2,
				},
				Token::Str("lat"),
				Token::F64(0.0),
				Token::Str("lng"),
				Token::F64(0.0),
				Token::StructEnd,
				Token::Str("createdCountry"),
				Token::Str("IN"),
				Token::Str("createdRegion"),
				Token::Str("Karnataka"),
				Token::Str("createdCity"),
				Token::Str("Bengaluru"),
				Token::Str("createdTimezone"),
				Token::Str("UTC"),
				Token::Str("lastLogin"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("lastActivity"),
				Token::Str("Fri, 15 May 2015 00:00:00 +0000"),
				Token::Str("lastActivityIp"),
				Token::Str("127.0.0.1"),
				Token::Str("lastActivityLocation"),
				Token::Struct {
					name: "Location",
					len: 2,
				},
				Token::Str("lat"),
				Token::F64(0.0),
				Token::Str("lng"),
				Token::F64(0.0),
				Token::StructEnd,
				Token::Str("lastActivityUserAgent"),
				Token::Str("user-agent"),
				Token::Str("lastActivityCountry"),
				Token::Str("IN"),
				Token::Str("lastActivityRegion"),
				Token::Str("Karnataka"),
				Token::Str("lastActivityCity"),
				Token::Str("Bengaluru"),
				Token::Str("lastActivityTimezone"),
				Token::Str("UTC"),
				Token::StructEnd,
				Token::SeqEnd,
				Token::MapEnd,
			],
		);
	}
}
