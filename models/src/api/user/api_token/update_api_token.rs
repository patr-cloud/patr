use std::collections::BTreeMap;

use ipnetwork::IpNetwork;
use time::OffsetDateTime;

use crate::{
	prelude::*,
	rbac::WorkspacePermission,
	utils::{constants::RESOURCE_NAME_REGEX, deserialize_double_option},
};

macros::declare_api_endpoint!(
	/// Update an API token. The older token will still be valid, but the details of the token will
	/// be updated.
	UpdateApiToken,
	PATCH "/user/api-token/{token_id}" {
		/// The ID of the token to update
		pub token_id: Uuid,
	},
	api = false,
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	request = {
		/// Change the name of the API token
		#[serde(skip_serializing_if = "Option::is_none")]
		#[preprocess(optional(trim, length(min = 4), regex = RESOURCE_NAME_REGEX))]
		pub name: Option<String>,
		/// Change the permissions of the API token
		#[serde(skip_serializing_if = "Option::is_none")]
		#[preprocess(none)]
		pub permissions: Option<BTreeMap<Uuid, WorkspacePermission>>,
		/// Change the time when the token becomes valid. Send `null` to clear
		/// any previously set NBF; omit the field to leave it unchanged.
		#[serde(default, deserialize_with = "deserialize_double_option")]
		#[serde(skip_serializing_if = "Option::is_none")]
		#[preprocess(none)]
		#[ts(type = "Date | null")]
		pub token_nbf: Option<Option<OffsetDateTime>>,
		/// Change the time when the token expires. Send `null` to clear any
		/// previously set EXP; omit the field to leave it unchanged.
		#[serde(default, deserialize_with = "deserialize_double_option")]
		#[serde(skip_serializing_if = "Option::is_none")]
		#[preprocess(none)]
		#[ts(type = "Date | null")]
		pub token_exp: Option<Option<OffsetDateTime>>,
		/// Change the list of allowed IPs for the token. Send `null` to clear
		/// the whitelist (= allow any IP); omit the field to leave it unchanged.
		#[serde(default, deserialize_with = "deserialize_double_option")]
		#[serde(skip_serializing_if = "Option::is_none")]
		#[preprocess(none)]
		#[ts(type = "Array<string> | null")]
		pub allowed_ips: Option<Option<Vec<IpNetwork>>>,
	},
	audit_log = NoAuditLogger,
);
