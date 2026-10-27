/// The endpoint to list the apps acting on a user's behalf
mod list_oauth_grants;
/// The endpoint to revoke an app's access
mod revoke_oauth_grant;

use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;

pub use self::{list_oauth_grants::*, revoke_oauth_grant::*};
use crate::prelude::*;

/// An app the user has let act on their behalf, from when they approve its
/// consent screen until they revoke it. The app's name and logo come from
/// `oauth_client`, so a grant still renders after its client leaves the
/// config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ListableResource, TS)]
#[serde(rename_all = "camelCase")]
pub struct UserOAuthGrant {
	/// The client the grant was given to.
	#[search(skip)]
	pub client_id: String,
	/// The app's name.
	pub client_name: String,
	/// The app's logo.
	#[search(skip)]
	pub client_logo_url: String,
	/// The app's website.
	#[search(skip)]
	pub client_uri: String,
	/// The identity scopes the user approved, space-delimited. These gate
	/// what the app can see about the user, not what it can do.
	#[search(skip)]
	pub scope: String,
	/// When the user approved it.
	#[ts(type = "Date")]
	#[search(skip)]
	pub created: OffsetDateTime,
	/// When the app last exchanged or refreshed a token.
	#[ts(type = "Date")]
	#[search(skip)]
	pub last_used: OffsetDateTime,
	/// Where the user approved it from.
	#[search(skip)]
	pub created_ip: IpAddr,
	/// The browser the user approved it in.
	#[search(skip)]
	pub created_user_agent: String,
}
