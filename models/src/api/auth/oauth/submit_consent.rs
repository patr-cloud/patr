use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Records the user's decision on a pending authorization request and
	/// returns where to send the browser: the client's redirect URI with a
	/// code, or with `access_denied`.
	SubmitConsent,
	POST "/auth/oauth/consent/{request_id}" {
		/// The opaque id of the request being decided.
		pub request_id: Uuid,
	},
	request_headers = {
		/// The authentication token of the user making the decision
		pub authorization: BearerToken,
		/// The user agent of the client
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	request = {
		/// Whether the user approved.
		#[preprocess(none)]
		pub approved: bool,
	},
	response = {
		/// Where to send the browser.
		pub redirect_uri: String,
	},
	client_type = [WebLogin],
	audit_log = NoAuditLogger,
);
