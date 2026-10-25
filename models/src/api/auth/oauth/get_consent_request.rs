use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Reads a pending authorization request for the consent screen. Doesn't
	/// consume it, so a reload still works.
	GetConsentRequest,
	GET "/auth/oauth/consent/{request_id}" {
		/// The id `/authorize` redirected the browser with.
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
		/// The client's display name.
		pub client_name: String,
		/// The client's icon.
		pub client_logo_url: String,
		/// The client's homepage.
		pub client_uri: String,
		/// The host the user will be sent back to.
		pub redirect_uri_host: String,
		/// The identity scopes being requested.
		pub scopes: Vec<String>,
		/// Whether the user already has a live grant for this client.
		pub previously_approved: bool,
	},
	client_type = [WebLogin],
	audit_log = NoAuditLogger,
);
