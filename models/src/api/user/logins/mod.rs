mod delete_login;
mod get_login_info;
mod list_user_logins;

use std::net::IpAddr;

use chrono::Utc;
use serde::{Deserialize, Serialize};

pub use self::{delete_login::*, get_login_info::*, list_user_logins::*};
use crate::utils::{DateTime, Location, Uuid};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserWebLogin {
	pub login_id: Uuid,
	pub token_expiry: DateTime<Utc>,
	pub created: DateTime<Utc>,
	pub created_ip: IpAddr,
	pub created_location: Location,
	pub created_country: String,
	pub created_region: String,
	pub created_city: String,
	pub created_timezone: String,
	pub last_login: DateTime<Utc>,
	pub last_activity: DateTime<Utc>,
	pub last_activity_ip: IpAddr,
	pub last_activity_location: Location,
	pub last_activity_user_agent: String,
	pub last_activity_country: String,
	pub last_activity_region: String,
	pub last_activity_city: String,
	pub last_activity_timezone: String,
}

#[cfg(test)]
mod test {
	use std::net::{IpAddr, Ipv4Addr};

	use chrono::{TimeZone, Utc};
	use serde_test::{assert_tokens, Configure, Token};

	use super::UserWebLogin;
	use crate::utils::{Location, Uuid};

	#[test]
	fn assert_user_login_types() {
		assert_tokens(
			&UserWebLogin {
				login_id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				token_expiry: Utc.timestamp_opt(1431648000, 0).unwrap().into(),
				created: Utc.timestamp_opt(1431648000, 0).unwrap().into(),
				created_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
				created_location: Location { lat: 0.0, lng: 0.0 },
				created_country: "IN".to_string(),
				created_region: "Karnataka".to_string(),
				created_city: "Bengaluru".to_string(),
				created_timezone: "UTC".to_string(),
				last_login: Utc.timestamp_opt(1431648000, 0).unwrap().into(),
				last_activity: Utc.timestamp_opt(1431648000, 0).unwrap().into(),
				last_activity_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
				last_activity_location: Location { lat: 0.0, lng: 0.0 },
				last_activity_user_agent: "user-agent".to_string(),
				last_activity_country: "IN".to_string(),
				last_activity_region: "Karnataka".to_string(),
				last_activity_city: "Bengaluru".to_string(),
				last_activity_timezone: "UTC".to_string(),
			}
			.readable(),
			&[
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
			],
		);
	}
}
