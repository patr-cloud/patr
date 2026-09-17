use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Deactivate multifactor authentication of a user
	DeactivateMfa,
	DELETE "/user/mfa",
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
		/// The one time password to deactivate mfa
		#[preprocess(none)]
		pub otp: String,
	},
	client_type = [WebDashboard],
	audit_log = NoAuditLogger,
);
