use std::collections::BTreeMap;

use ipnetwork::IpNetwork;
use time::OffsetDateTime;

use crate::{permission::WorkspacePermission, prelude::*};

macros::declare_api_endpoint!(
	/// Update an API token. The older token will still be valid, but the details of the token will
	/// be updated.
	UpdateApiToken,
	PATCH "/user/api-token/:token_id" {
		/// The ID of the token to update
		pub token_id: Uuid,
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	request = {
		/// Change the name of the API token
		#[serde(skip_serializing_if = "Option::is_none")]
		pub name: Option<String>,
		/// Change the permissions of the API token
		#[serde(skip_serializing_if = "Option::is_none")]
		pub permissions: Option<BTreeMap<Uuid, WorkspacePermission>>,
		/// Change the time when the token becomes valid
		#[serde(skip_serializing_if = "Option::is_none")]
		pub token_nbf: Option<OffsetDateTime>,
		/// Change the time when the token expires
		#[serde(skip_serializing_if = "Option::is_none")]
		pub token_exp: Option<OffsetDateTime>,
		/// Change the list of allowed IPs for the token
		#[serde(skip_serializing_if = "Option::is_none")]
		pub allowed_ips: Option<Vec<IpNetwork>>,
	}
);
