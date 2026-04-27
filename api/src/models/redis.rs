use std::{collections::BTreeMap, net::IpAddr};

use models::rbac::WorkspacePermission;
use semver::Version;
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

/// State for an in-flight runner setup, stored in Redis keyed by
/// [`crate::redis::keys::runner_setup_data`]. The CLI creates the entry,
/// the browser approves it, the CLI claims credentials on its next verify
/// poll.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSetupDataEntry {
	/// 32-byte opaque secret. Constant-time-compared against the value the
	/// CLI sends on `POST /runner/verify`.
	pub device_code: String,
	/// CLI-reported runner version (semver).
	pub version: Version,
	/// CLI-reported OS string.
	pub os: String,
	/// CLI-reported CPU architecture.
	pub arch: String,
	/// CLI-reported hostname.
	pub hostname: String,
	/// Public IP the server saw on the create request.
	pub public_ip: IpAddr,
	/// CLI-reported private IP.
	pub private_ip: IpAddr,
	/// City resolved from the public IP via ipinfo (None on lookup failure).
	pub city: Option<String>,
	/// Country resolved from the public IP via ipinfo.
	pub country: Option<String>,
	/// Latitude resolved from the public IP via ipinfo.
	pub latitude: Option<f64>,
	/// Longitude resolved from the public IP via ipinfo.
	pub longitude: Option<f64>,
	/// When the link was created.
	pub created_at: OffsetDateTime,
	/// Set by the browser approve handler. Until then, verify polls return
	/// `Pending`. Once set, the next verify poll returns `Approved` and
	/// deletes the entry.
	pub approved: Option<RunnerApprovedSetupData>,
}

/// Credentials issued when a runner setup is approved.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerApprovedSetupData {
	/// ID of the runner that was created.
	pub runner_id: Uuid,
	/// Workspace the runner was added to.
	pub workspace_id: Uuid,
	/// Service account token (`patrv1.{refresh_token}.{sa_id}`).
	pub token: String,
}
