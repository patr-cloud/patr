use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Activate multifactor authentication of a user
	ActivateMfa,
	POST "/user/mfa",
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user agent of the client making the request
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	request = {
		/// The one time password to activate mfa
		#[preprocess(none)]
		pub otp: String,
	},
	client_type = [WebDashboard],
	audit_log = NoAuditLogger,
);
