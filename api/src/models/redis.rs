use std::collections::BTreeMap;

use models::rbac::WorkspacePermission;
use serde::{Deserialize, Serialize};
use sqlx::types::ipnetwork::IpNetwork;
use time::OffsetDateTime;

use crate::prelude::*;

/// Everything an authenticated request needs to know about the actor behind
/// a login, as cached in Redis under
/// [`auth_data_for_login_id`][redis::keys::auth_data_for_login_id]. A cache
/// hit builds the request's [`RequestActorData`] from this alone, without
/// touching the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorAuthDataCache {
	/// The actor behind the login: the user ID or the service account ID.
	pub actor_id: Uuid,
	/// Which kind of login this is, and what it carries.
	pub kind: ActorAuthDataCacheKind,
	/// The actor's permissions on every workspace it belongs to.
	pub permissions: BTreeMap<Uuid, WorkspacePermission>,
	/// When this entry's data was read from the database. Compared against
	/// the `*_cache_stale_since` stamps: any stamp newer than this makes the
	/// entry stale.
	pub created_at: OffsetDateTime,
}

/// The kind of login an [`ActorAuthDataCache`] entry describes, with the
/// actor's details for that kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ActorAuthDataCacheKind {
	/// A user's web dashboard session.
	#[serde(rename_all = "camelCase")]
	WebLogin {
		/// The user's email.
		email: String,
		/// The user's first name.
		first_name: String,
		/// The user's last name.
		last_name: String,
		/// When the user was created.
		created: OffsetDateTime,
	},
	/// A user's API token.
	#[serde(rename_all = "camelCase")]
	ApiToken {
		/// The user's email.
		email: String,
		/// The user's first name.
		first_name: String,
		/// The user's last name.
		last_name: String,
		/// When the user was created.
		created: OffsetDateTime,
		/// The networks the token may be used from, if restricted. Kept in
		/// the entry because the client IP differs per request and has to be
		/// checked on cache hits too.
		allowed_ips: Option<Vec<IpNetwork>>,
		/// The argon2 hash of the token's secret, as stored in the database,
		/// so the presented secret can be verified on a hit without a
		/// database round trip.
		token_hash: String,
	},
	/// A service account's token.
	#[serde(rename_all = "camelCase")]
	ServiceAccount {
		/// The service account's name.
		name: String,
		/// When the service account was created.
		created: OffsetDateTime,
		/// The argon2 hash of the token's secret, as stored in the database,
		/// so the presented secret can be verified on a hit without a
		/// database round trip.
		token_hash: String,
	},
}
