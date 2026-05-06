use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Get the version of the server.
	GetVersion,
	GET "/version",
	workspaced = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	audit_log = NoAuditLogger,
);
