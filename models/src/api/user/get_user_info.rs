use time::OffsetDateTime;

use super::BasicUserInfo;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Get the information of the currently authenticated user.
	GetUserInfo,
	GET "/user",
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
		/// When the user account was created
		#[ts(type = "Date")]
		pub created: OffsetDateTime,
		/// The primary email of the user
		pub email: String,
		/// Check if MFA is enabled or not
		pub is_mfa_enabled: bool,
	},
	audit_log = NoAuditLogger,
);
