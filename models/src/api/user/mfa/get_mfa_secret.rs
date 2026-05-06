use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Get a mfa secret which will be used for verification
	GetMfaSecret,
	GET "/user/mfa",
	workspaced = false,
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user agent of the client making the request
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	response = {
		/// The MFA secret QR code URL
		pub qr: String,
	},
	audit_log = NoAuditLogger,
);
