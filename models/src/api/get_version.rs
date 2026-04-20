use semver::Version;

use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Get the version of the server. Used by the web dashboard to compare
	/// against each runner's reported version for the "outdated" indicator.
	GetVersion,
	GET "/version",
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	response = {
		/// The semver version of the running API build
		#[ts(type = "string")]
		pub version: Version,
	},
	client_type = [WebDashboard, ApiToken],
	audit_log = NoAuditLogger,
);
