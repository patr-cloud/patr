use crate::{prelude::*, utils::constants::RESOURCE_NAME_REGEX};

macros::declare_api_endpoint!(
	/// Route to check if a workspace name is available
	IsWorkspaceNameAvailable,
	GET "/workspace/name-available",
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	query = {
		/// The name of the workspace to check
		#[preprocess(trim, regex = RESOURCE_NAME_REGEX)]
		pub name: String,
	},
	response = {
		/// Whether the workspace name is available
		pub available: bool,
	},
	audit_log = NoAuditLogger,
);
