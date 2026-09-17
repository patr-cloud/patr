use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Records the user's decision on a pending authorization request, and
	/// returns the URL to send the browser to.
	///
	/// Consuming: the request is gone afterwards either way, so a decision
	/// cannot be replayed.
	///
	/// The response URL is built entirely server-side. Validating the
	/// `redirect_uri` and echoing the client's `state` are security-critical,
	/// so the page is given somewhere to navigate to rather than the pieces
	/// to assemble one from.
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
		/// Whether the user approved. A denial still produces a redirect —
		/// the client is told `access_denied` rather than being left to time
		/// out.
		#[preprocess(none)]
		pub approved: bool,
	},
	response = {
		/// Where to send the browser. Carries the authorization code on
		/// approval, or `error=access_denied` on refusal, with the client's
		/// `state` echoed either way.
		pub redirect_uri: String,
	},
	audit_log = NoAuditLogger,
	api = false,
);
