/// The endpoint to list every app currently acting on a user's behalf
mod list_oauth_grants;
/// The endpoint to revoke a single grant
mod revoke_oauth_grant;
/// The endpoint to revoke every grant a user holds for one app
mod revoke_oauth_grants_for_client;

use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;

pub use self::{list_oauth_grants::*, revoke_oauth_grant::*, revoke_oauth_grants_for_client::*};
use crate::prelude::*;

/// One app's standing permission to act as the user.
///
/// The answer to "who is acting on my behalf, and how do I stop them". A grant
/// is created when the user approves a consent screen and lives until they
/// revoke it, so unlike a web login there is no expiry to show — an app either
/// still has access or does not.
///
/// The client's display metadata is joined in from `oauth_client` rather than
/// read from the config, so a grant still renders with a name and a logo after
/// its client has been pulled from the config. Which is the point of that
/// table existing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ListableResource, TS)]
#[serde(rename_all = "camelCase")]
pub struct UserOAuthGrant {
	/// The client this grant was given to.
	pub client_id: String,
	/// The app's display name, for the authorized-apps screen.
	pub client_name: String,
	/// The app's logo.
	pub client_logo_url: String,
	/// The app's own website.
	pub client_uri: String,
	/// The identity scopes the user approved, space-delimited. What the app
	/// can see about them — never what it can do, which is always their full
	/// permissions.
	pub scope: String,
	/// When the user approved this.
	#[ts(type = "Date")]
	pub created: OffsetDateTime,
	/// When the app last exchanged or refreshed a token. The closest thing to
	/// "is this still in use".
	#[ts(type = "Date")]
	pub last_used: OffsetDateTime,
	/// The IP the consent screen was approved from. Unlike a web login there
	/// is no geo here: a grant is approved once from a browser and then used
	/// server-to-server, so a location would describe the consent moment
	/// rather than where the app is calling from.
	pub created_ip: IpAddr,
	/// The browser the user approved it in.
	pub created_user_agent: String,
}
