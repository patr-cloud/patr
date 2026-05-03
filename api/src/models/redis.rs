use std::{collections::BTreeMap, net::IpAddr};

use models::rbac::WorkspacePermission;
use semver::Version;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::prelude::*;

/// The struct that is used to insert a user's permissions into Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPermissionCache {
	/// The user's permissions
	pub permission: BTreeMap<Uuid, WorkspacePermission>,
	/// The timestamp when the user's permissions were inserted into Redis
	pub creation_time: OffsetDateTime,
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
