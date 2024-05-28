use crate::{api::user::UserPhoneNumber, prelude::*};

macros::declare_api_endpoint!(
	/// Verify the recovery phone number for the currently authenticated user. This endpoint is
	/// used to verify the phone number after the user has requested to change their phone number
	/// using the [`super::update_user_phone_number`] endpoint.
	VerifyUserPhoneNumber,
	POST "/user/verify-phone-number",
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
		/// The new phone number
		#[preprocess(none)]
		pub phone_number: UserPhoneNumber,
		/// The verification token sent to the new phone number
		#[preprocess(none)]
		pub verification_token: String,
	},
);
