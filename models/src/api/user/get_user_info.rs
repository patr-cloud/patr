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
		/// Alternate emails of the user, if any
		pub secondary_emails: Vec<String>,
		/// The primary phone number of the user
		pub recovery_phone_number: Option<UserPhoneNumber>,
		/// Alternate phone numbers of the user, if any
		pub secondary_phone_numbers: Vec<UserPhoneNumber>,
	}
);
