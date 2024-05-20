use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Update the email for the currently authenticated user. An OTP will be sent to
	/// the new email address. The user then must verify the new email address using the
	/// [`super::verify_email_address`] endpoint.
	UpdateUserEmail,
	POST "/user/update-email",
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
		/// The new email. A `None` value will remove the email. This is only
		/// allowed if the user has a phone number.
		// #[preprocess(email)]
		pub email: Option<String>,
	},
);
