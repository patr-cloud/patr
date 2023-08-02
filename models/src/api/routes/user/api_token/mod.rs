mod create_api_token;
mod list_all_api_tokens;
mod list_permissions_for_api_token;
mod regenerate_api_token;
mod revoke_api_token;
mod update_api_token;

use chrono::Utc;
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};

pub use self::{
	create_api_token::*,
	list_all_api_tokens::*,
	list_permissions_for_api_token::*,
	regenerate_api_token::*,
	revoke_api_token::*,
	update_api_token::*,
};
use crate::utils::{DateTime, Uuid};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserApiToken {
	pub id: Uuid,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub token_nbf: Option<DateTime<Utc>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub token_exp: Option<DateTime<Utc>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub allowed_ips: Option<Vec<IpNetwork>>,
	pub created: DateTime<Utc>,
}

#[cfg(test)]
mod test {
	use std::str::FromStr;

	use chrono::{TimeZone, Utc};
	use ipnetwork::IpNetwork;
	use serde_test::{assert_tokens, Token};

	use super::UserApiToken;
	use crate::utils::Uuid;

	#[test]
	fn assert_empty_user_api_token_types() {
		assert_tokens(
			&UserApiToken {
				id: Uuid::parse_str("2aef18631ded45eb9170dc2166b30867")
					.unwrap(),
				name: "Token 1".to_string(),
				token_nbf: None,
				token_exp: None,
				allowed_ips: None,
				created: Utc.timestamp_opt(1431648000, 0).unwrap().into(),
			},
			&[
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
			],
		);
	}

	#[test]
	fn assert_filled_user_api_token_types() {
		assert_tokens(
			&UserApiToken {
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
				created: Utc.timestamp_opt(1431648000, 0).unwrap().into(),
			},
			&[
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
			],
		);
	}
}
