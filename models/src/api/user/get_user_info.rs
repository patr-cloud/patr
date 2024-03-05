use time::OffsetDateTime;

use super::{BasicUserInfo, UserPhoneNumber};
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
		/// The basic info of the user. Username, userId, first name, last name, etc.
		#[serde(flatten)]
		pub basic_user_info: WithId<BasicUserInfo>,
		/// When the user account was created
		pub created: OffsetDateTime,
		/// The primary recovery email of the user
		pub recovery_email: Option<String>,
		/// The primary phone number of the user
		pub recovery_phone_number: Option<UserPhoneNumber>,
		/// Check if MFA is enabled or not
		pub is_mfa_enabled: bool,
	}
);
