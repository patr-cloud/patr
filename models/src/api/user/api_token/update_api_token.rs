use super::UserApiToken;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Update an API token. The older token will still be valid, but the details of the token will
	/// be updated.
	UpdateApiToken,
	PATCH "/user/api-token/{token_id}" {
		/// The ID of the token to update
		pub token_id: Uuid,
	},
	api = false,
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	request = {
		/// The updated token details
		#[serde(flatten)]
		#[preprocess]
		pub token: UserApiToken,
	},
	audit_log = NoAuditLogger,
);
