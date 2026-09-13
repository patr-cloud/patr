use super::OAuthScopeInfo;
use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Reads the pending authorization request that a consent screen is
	/// about to render.
	///
	/// Deliberately does **not** consume the request: the page may be
	/// reloaded, and server-side rendering plus hydration can both reach for
	/// it. Only [`SubmitConsent`][1] resolves it.
	///
	/// [1]: super::SubmitConsentRequest
	GetConsentRequest,
	GET "/auth/oauth/consent/{request_id}" {
		/// The opaque id handed to the browser by the authorization
		/// endpoint. It names a request parked server-side, so the client's
		/// own parameters never reach the dashboard's address bar.
		pub request_id: Uuid,
	},
	request_headers = {
		/// The authentication token of the user being asked to consent
		pub authorization: BearerToken,
		/// The user agent of the client
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	response = {
		/// The client asking for access.
		pub client_id: String,
		/// The client's display name, as the consent screen should show it.
		pub client_name: String,
		/// The client's icon.
		pub client_logo_url: String,
		/// The client's homepage.
		pub client_uri: String,
		/// The host the user will be sent back to. Shown so they can see
		/// where they are about to be returned to.
		pub redirect_uri_host: String,
		/// The identity scopes being requested.
		pub scopes: Vec<OAuthScopeInfo>,
		/// Whether this user has already approved this client with these
		/// scopes, so the screen can say "reconnecting" rather than
		/// presenting it as a first-time decision.
		pub previously_approved: bool,
	},
	audit_log = NoAuditLogger,
	api = false,
);
