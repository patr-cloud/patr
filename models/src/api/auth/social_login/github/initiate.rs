use crate::{api::auth::SocialLoginProvider, prelude::*};

macros::declare_api_endpoint!(
	/// Initiates the social-login OAuth flow. Generates a CSRF state token,
	/// stores it in Redis for 10 minutes, and returns the full authorization
	/// URL that the frontend should redirect the browser to.
	SocialLoginInitiate,
	POST "/auth/social-login/{provider}" {
		/// The social-login provider to initiate. Must be `github` for now.
		pub provider: SocialLoginProvider,
	},
	client_type = [WebDashboard],
	response = {
		/// The full provider authorization URL. The frontend must redirect
		/// the user's browser to this URL to begin the OAuth flow.
		pub authorize_url: String,
	},
	audit_log = NoAuditLogger,
);
