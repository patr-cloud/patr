use crate::{api::user::UserPhoneNumber, prelude::*};

macros::declare_api_endpoint!(
	/// Update the phone number for the currently authenticated user. An OTP will be sent
	/// to the new phone number. The user then must verify the new phone number using the
	/// [`super::verify_phone_number`] endpoint.
	UpdateUserPhoneNumber,
	POST "/user/update-phone-number",
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
		/// The new phone number. A `None` value will remove the phone number. This is only
		/// allowed if the user has an email address
		#[preprocess(none)]
		pub phone_number: Option<UserPhoneNumber>,
	},
);
