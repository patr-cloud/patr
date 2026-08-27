use super::BasicUserInfo;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Get a user's details by userId. This will return the user's basic info,
	/// such as their first name and last name.
	GetUserDetails,
	GET "/user/{user_id}" {
		/// The userId of the user whose details are being requested.
		pub user_id: Uuid,
	},
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	response = {
		/// The basic info of the user.
		#[serde(flatten)]
		pub basic_user_info: WithId<BasicUserInfo>,
	},
	audit_log = NoAuditLogger,
);
