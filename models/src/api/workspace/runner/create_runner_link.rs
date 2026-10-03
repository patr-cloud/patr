use std::net::IpAddr;

use semver::Version;

use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Begin a runner consent-link flow. Called by the CLI before the user
	/// approves the link in their browser. The CLI then polls
	/// `POST /workspace/{workspace_id}/runner/link/verify` with the returned
	/// `device_code` until the link is approved.
	CreateRunnerLink,
	POST "/workspace/{workspace_id}/runner/link" {
		/// Workspace to add the runner to. The CLI takes this from its
		/// current_workspace state (with a `Select` fallback if unset).
		pub workspace_id: Uuid,
	},
	request_headers = {
		/// Bearer token for the logged-in user driving the CLI.
		pub authorization: BearerToken,
		/// The user-agent of the CLI making the request.
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::ResourcePermissionAuthenticator {
			extract_resource_id: |req| req.path.workspace_id,
			extract_workspace_id: |req| req.path.workspace_id,
			permission: Permission::Runner(RunnerPermission::Create),
		}
	},
	request = {
		/// Runner version (semver) shown on the consent page.
		#[ts(type = "string")]
		pub version: Version,
		/// OS string from `std::env::consts::OS` (e.g. "linux", "macos").
		pub os: String,
		/// CPU architecture from `std::env::consts::ARCH` (e.g. "x86_64").
		pub arch: String,
		/// Hostname of the machine running the CLI.
		pub hostname: String,
		/// Private IP the CLI sees on its primary interface. The server
		/// already learns the public IP from the request, but the private
		/// IP needs the CLI to report it.
		pub private_ip: IpAddr,
	},
	response = {
		/// 8-char base32 user-typeable code shown on the consent page and
		/// passed via `?code=` in the verification URL.
		pub user_code: String,
		/// 32-byte opaque secret returned only to the CLI. The browser
		/// never sees it; the CLI sends it back when polling
		/// `POST /workspace/{workspace_id}/runner/link/verify` to claim
		/// credentials.
		pub device_code: String,
		/// Browser URL where the user enters the user code manually.
		pub verification_uri: String,
		/// Same URL with `?code=...` prefilled — used by the CLI to launch
		/// the browser directly.
		pub verification_uri_complete: String,
		/// Seconds until the link expires server-side.
		pub expires_in: u64,
		/// Seconds the CLI should wait between verify polls.
		pub interval: u64,
	},
	client_type = [ApiToken],
	audit_log = NoAuditLogger,
);
