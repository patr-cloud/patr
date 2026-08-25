use std::collections::{BTreeMap, BTreeSet};

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
use crate::api::workspace::rbac::user::RoleGrant;

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ListableResource, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct UserApiToken {
	/// A user-friendly name for the token. This is used to identify the token
	/// when the user is looking at the list of tokens.
	#[preprocess(trim, length(min = 4), regex = RESOURCE_NAME_REGEX)]
	pub name: String,
	/// The workspaces this token has super-admin access to. Only the
	/// workspace's owner can mint these.
	#[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
	#[search(skip)]
	pub super_admin_of: BTreeSet<Uuid>,
	/// The token's role grants per workspace. These are a ceiling, not a
	/// grant: the token's effective permissions are this intersected with
	/// its owner's current permissions, computed at auth time. A ceiling
	/// above the owner's reach is allowed and clamps harmlessly.
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	#[search(skip)]
	pub grants: BTreeMap<Uuid, Vec<RoleGrant>>,
	/// Any token that is used before the nbf (not before) should be rejected.
	/// Tokens are only valid after this time.
	#[serde(
		default,
		skip_serializing_if = "Option::is_none",
		with = "time::serde::rfc3339::option"
	)]
	#[ts(type = "Date")]
	pub token_nbf: Option<OffsetDateTime>,
	/// Any token that is used after the exp (expiry) should be rejected. Tokens
	/// are only valid before this time.
	#[serde(
		default,
		skip_serializing_if = "Option::is_none",
		with = "time::serde::rfc3339::option"
	)]
	#[ts(type = "Date")]
	pub token_exp: Option<OffsetDateTime>,
	/// The IP addresses that are allowed to use this token. If this is not
	/// specified, then any IP address can use this token. This can also take a
	/// CIDR range, to allow a range of IP addresses.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	#[ts(type = "Array<string>")]
	pub allowed_ips: Option<Vec<IpNetwork>>,
	/// The time at which this token was created.
	#[serde(default = "default_created")]
	#[ts(type = "Date")]
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
	use serde_test::{Configure, Token, assert_tokens};
	use time::OffsetDateTime;

	use super::UserApiToken;
	use crate::{api::workspace::rbac::user::RoleGrant, prelude::*, rbac::PermissionScope};

	#[test]
	fn assert_empty_user_api_token_types() {
		assert_tokens(
			&UserApiToken {
				name: "Token 1".to_string(),
				super_admin_of: Default::default(),
				grants: Default::default(),
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
				super_admin_of: BTreeSet::from([Uuid::nil()]),
				grants: BTreeMap::from([(
					Uuid::parse_str("00000000000000000000000000000001").unwrap(),
					vec![RoleGrant {
						role_id: Uuid::nil(),
						scope: PermissionScope::Resources(BTreeSet::from([Uuid::nil()])),
					}],
				)]),
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
					len: 7,
				},
				Token::Str("name"),
				Token::Str("Token 2"),
				Token::Str("superAdminOf"),
				Token::Seq { len: Some(1) },
				Token::Str("00000000000000000000000000000000"),
				Token::SeqEnd,
				Token::Str("grants"),
				Token::Map { len: Some(1) },
				Token::Str("00000000000000000000000000000001"),
				Token::Seq { len: Some(1) },
				Token::Struct {
					name: "RoleGrant",
					len: 2,
				},
				Token::Str("roleId"),
				Token::Str("00000000000000000000000000000000"),
				Token::Str("scope"),
				Token::Struct {
					name: "PermissionScope",
					len: 2,
				},
				Token::Str("scopeType"),
				Token::UnitVariant {
					name: "PermissionScope",
					variant: "resources",
				},
				Token::Str("resources"),
				Token::Seq { len: Some(1) },
				Token::Str("00000000000000000000000000000000"),
				Token::SeqEnd,
				Token::StructEnd,
				Token::StructEnd,
				Token::SeqEnd,
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
