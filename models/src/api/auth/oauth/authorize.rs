use crate::prelude::*;

macros::declare_api_endpoint!(
	/// The authorization endpoint: where a client sends the browser to start
	/// a login.
	///
	/// Validates what it can without a session, parks the request in Redis
	/// and bounces the browser to the dashboard's consent screen carrying an
	/// opaque request id. The user is bound there, from their own session —
	/// this endpoint is served from the API's origin and cannot read it.
	///
	/// The query keeps OAuth's `snake_case`, since the client's library
	/// builds it.
	OAuthAuthorize,
	GET "/auth/oauth/authorize",
	query = {
		/// The client asking.
		#[serde(rename = "client_id")]
		pub client_id: String,
		/// Where to send the browser afterwards. Must match one the client
		/// registered.
		#[serde(rename = "redirect_uri")]
		pub redirect_uri: Option<String>,
		/// Must be `code`.
		#[serde(rename = "response_type")]
		pub response_type: Option<String>,
		/// Space-separated identity scopes.
		pub scope: Option<String>,
		/// The client's opaque CSRF token, echoed back untouched.
		pub state: Option<String>,
		/// The OIDC nonce, bound into the id token.
		pub nonce: Option<String>,
		/// The PKCE challenge.
		#[serde(rename = "code_challenge")]
		pub code_challenge: Option<String>,
		/// Must be `S256`.
		#[serde(rename = "code_challenge_method")]
		pub code_challenge_method: Option<String>,
	},
	response_headers = {
		/// Where the browser is sent next: the consent screen, or back to
		/// the client with an error.
		pub location: Location,
	},
	audit_log = NoAuditLogger,
);
