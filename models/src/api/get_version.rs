use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Get the version of the server.
	GetVersion,
	GET "/version",
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	client_type = [WebDashboard, ApiToken],
	audit_log = NoAuditLogger,
);
