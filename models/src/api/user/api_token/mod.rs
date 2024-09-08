use std::collections::BTreeMap;

use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

/// The endpoint to create an API token
mod create_api_token;
/// The endpoint to get the information of an API token
mod get_api_token_info;
/// The endpoint to list all the API tokens of a user
mod list_api_tokens;
/// The endpoint to regenerate an API token
mod regenerate_api_token;
/// The endpoint to revoke an API token
mod revoke_api_token;
/// The endpoint to update an API token
mod update_api_token;

use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub use self::{
	create_api_token::*,
	get_api_token_info::*,
	list_api_tokens::*,
	regenerate_api_token::*,
	revoke_api_token::*,
	update_api_token::*,
};
use crate::rbac::WorkspacePermission;

#[::preprocess::sync]
/// An API token created by the user.
///
/// This is mostly used by the user if they want to automate something on Patr
/// using the API. The ID of the token is the same as the login ID. The only
/// problem here is that since login IDs are hard-coded in the API token, we
/// will have to explicitly store the IP address and other things in the audit
/// log to make sure that we can track the token, instead of changing the
/// loginId when something changes. Not sure how to go about doing that yet.
///
/// I mean, if we're anyway gonna store everything in the audit log, then why
/// store anything in the login ID table? Ehh, idk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserApiToken {
	/// A user-friendly name for the token. This is used to identify the token
	/// when the user is looking at the list of tokens.
	#[preprocess(trim, length(min = 4), regex = RESOURCE_NAME_REGEX)]
	pub name: String,
	/// The list of permissions for this token for a given workspace. A token
	/// can have multiple permissions across different workspaces. But all the
	/// actions performed by the token will be logged as the user who created
	/// the token.
	#[serde(skip_serializing_if = "BTreeMap::is_empty")]
	#[serde(default)]
	pub permissions: BTreeMap<Uuid, WorkspacePermission>,
	/// Any token that is used before the nbf (not before) should be rejected.
	/// Tokens are only valid after this time.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub token_nbf: Option<OffsetDateTime>,
	/// Any token that is used after the exp (expiry) should be rejected. Tokens
	/// are only valid before this time.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub token_exp: Option<OffsetDateTime>,
	/// The IP addresses that are allowed to use this token. If this is not
	/// specified, then any IP address can use this token. This can also take a
	/// CIDR range, to allow a range of IP addresses.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub allowed_ips: Option<Vec<IpNetwork>>,
	/// The time at which this token was created.
	#[serde(default = "default_created")]
	pub created: OffsetDateTime,
}

/// The default value for the `created` field of the `UserApiToken` struct. This
/// value currently defaults to the UNIX epoch (1970-01-01 00:00:00 UTC).
const fn default_created() -> OffsetDateTime {
	OffsetDateTime::UNIX_EPOCH
}

#[cfg(test)]
mod test {
	use std::{
		collections::{BTreeMap, BTreeSet},
		str::FromStr,
	};

	use ipnetwork::IpNetwork;
	use serde_test::{assert_tokens, Configure, Token};
	use time::OffsetDateTime;

	use super::UserApiToken;
	use crate::{
		prelude::*,
		rbac::{ResourcePermissionType, WorkspacePermission},
	};

	#[test]
	fn assert_empty_user_api_token_types() {
		assert_tokens(
			&UserApiToken {
				name: "Token 1".to_string(),
				permissions: Default::default(),
				token_nbf: None,
				token_exp: None,
				allowed_ips: None,
				created: OffsetDateTime::UNIX_EPOCH,
			}
			.readable(),
			&[
				Token::Struct {
					name: "UserApiToken",
					len: 2,
				},
				Token::Str("name"),
				Token::Str("Token 1"),
				Token::Str("created"),
				Token::Str("1970-01-01 00:00:00.0 +00:00:00"),
				Token::StructEnd,
			],
		);
	}

	#[test]
	fn assert_filled_user_api_token_types() {
		assert_tokens(
			&UserApiToken {
				name: "Token 2".to_string(),
				permissions: {
					let mut map = BTreeMap::new();
					map.insert(Uuid::nil(), WorkspacePermission::SuperAdmin);
					map.insert(
						Uuid::parse_str("00000000000000000000000000000001").unwrap(),
						WorkspacePermission::Member {
							permissions: {
								let mut map = BTreeMap::new();
								map.insert(
									Uuid::nil(),
									ResourcePermissionType::Include(BTreeSet::from([Uuid::nil()])),
								);
								map
							},
						},
					);
					map
				},
				token_nbf: Some(OffsetDateTime::UNIX_EPOCH),
				token_exp: Some(OffsetDateTime::UNIX_EPOCH),
				allowed_ips: Some(vec![
					IpNetwork::from_str("1.1.1.1").unwrap(),
					IpNetwork::from_str("1.0.0.0/8").unwrap(),
				]),
				created: OffsetDateTime::UNIX_EPOCH,
			}
			.readable(),
			&[
				Token::Struct {
					name: "UserApiToken",
					len: 6,
				},
				Token::Str("name"),
				Token::Str("Token 2"),
				Token::Str("permissions"),
				Token::Map { len: Some(2) },
				Token::Str("00000000000000000000000000000000"),
				Token::Struct {
					name: "WorkspacePermission",
					len: 1,
				},
				Token::Str("type"),
				Token::Str("superAdmin"),
				Token::StructEnd,
				Token::Str("00000000000000000000000000000001"),
				Token::Map { len: None },
				Token::Str("type"),
				Token::Str("member"),
				Token::Str("00000000000000000000000000000000"),
				Token::Struct {
					name: "ResourcePermissionType",
					len: 2,
				},
				Token::Str("permissionType"),
				Token::UnitVariant {
					name: "ResourcePermissionType",
					variant: "include",
				},
				Token::Str("resources"),
				Token::Seq { len: Some(1) },
				Token::Str("00000000000000000000000000000000"),
				Token::SeqEnd,
				Token::StructEnd,
				Token::MapEnd,
				Token::MapEnd,
				Token::Str("tokenNbf"),
				Token::Some,
				Token::Str("1970-01-01 00:00:00.0 +00:00:00"),
				Token::Str("tokenExp"),
				Token::Some,
				Token::Str("1970-01-01 00:00:00.0 +00:00:00"),
				Token::Str("allowedIps"),
				Token::Some,
				Token::Seq { len: Some(2) },
				Token::Str("1.1.1.1/32"),
				Token::Str("1.0.0.0/8"),
				Token::SeqEnd,
				Token::Str("created"),
				Token::Str("1970-01-01 00:00:00.0 +00:00:00"),
				Token::StructEnd,
			],
		);
	}
}
