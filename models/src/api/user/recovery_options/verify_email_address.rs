use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Verify the email for the currently authenticated user. This endpoint is used to
	/// verify the email address after the user has requested to change their email
	/// using the [`super::update_user_email`] endpoint.
	VerifyUserEmail,
	POST "/user/verify-email",
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	request = {
		/// The new email address
		pub email: String,
		/// The verification token sent to the new email address
		pub verification_token: String,
	},
);
