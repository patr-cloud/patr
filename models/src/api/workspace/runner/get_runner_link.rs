use std::net::IpAddr;

use semver::Version;

use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Fetch the metadata for a runner link, used to populate the consent
	/// page in the browser. Returns the same details the CLI reported when
	/// creating the link, plus the public IP and geolocation the server
	/// resolved at creation time.
	GetRunnerLink,
	GET "/workspace/{workspace_id}/runner/link/{user_code}" {
		/// Workspace currently selected in the browser. Must match the
		/// workspace the link was created in (otherwise 404 — the consent
		/// page will nudge the user to switch workspaces).
		pub workspace_id: Uuid,
		/// The user-typeable code from the verification URL.
		pub user_code: String,
	},
	request_headers = {
		/// Bearer token for the logged-in user.
		pub authorization: BearerToken,
		/// The user-agent of the browser.
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Runner(RunnerPermission::Create),
		}
	},
	response = {
		/// Runner version (semver).
		#[ts(type = "string")]
		pub version: Version,
		/// OS string the CLI reported.
		pub os: String,
		/// CPU architecture the CLI reported.
		pub arch: String,
		/// Hostname the CLI reported.
		pub hostname: String,
		/// Public IP the server saw on the create_link request.
		#[ts(type = "string")]
		pub public_ip: IpAddr,
		/// Private IP the CLI reported.
		#[ts(type = "string")]
		pub private_ip: IpAddr,
		/// City resolved from the public IP via ipinfo.
		pub city: Option<String>,
		/// Country resolved from the public IP via ipinfo.
		pub country: Option<String>,
		/// Latitude resolved from the public IP via ipinfo.
		pub latitude: Option<f64>,
		/// Longitude resolved from the public IP via ipinfo.
		pub longitude: Option<f64>,
		/// When the link was created (server time, RFC 3339).
		#[ts(type = "Date")]
		pub created_at: time::OffsetDateTime,
	},
	client_type = [WebDashboard],
	audit_log = NoAuditLogger,
);
