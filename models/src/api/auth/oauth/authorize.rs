use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Where a client sends the browser to start a login. Parks the request
	/// and redirects to the consent screen with its id. The query is
	/// `snake_case`, as OAuth defines it.
	OAuthAuthorize,
	GET "/auth/oauth/authorize",
	query = {
		/// The client asking.
		#[serde(rename = "client_id")]
		pub client_id: String,
		/// Where to send the browser afterwards.
		#[serde(rename = "redirect_uri")]
		pub redirect_uri: Option<String>,
		/// Must be `code`.
		#[serde(rename = "response_type")]
		pub response_type: Option<String>,
		/// Space-separated identity scopes.
		pub scope: Option<String>,
		/// The client's CSRF token, echoed back untouched.
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
		/// The consent screen, or the client's redirect URI with an error.
		pub location: Location,
	},
	client_type = [OAuthLogin],
	audit_log = NoAuditLogger,
);
