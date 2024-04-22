use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Activate multifactor authentication of a user
	ActivateMfa,
	POST "/user/mfa",
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	request = {
		/// The one time password to activate mfa
		#[preprocess(none)]
		pub otp: String,
	},
);
