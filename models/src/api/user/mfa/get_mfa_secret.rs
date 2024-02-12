use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Get a mfa secret which will be used for verification
	GetMfaSecret,
	GET "/user/mfa",
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	response = {
		/// The MFA secret used to verify
		pub secret: String,
	},
);
