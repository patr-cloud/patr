use crate::{api::auth::oauth::CodeChallengeHashMethod, prelude::*};

macros::declare_api_endpoint!(
	/// POST /auth/oauth/authorize
   ///
   /// User submits login credentials. Backend validates:
   /// - user credentials
   /// - client_id & redirect_uri match
   /// - PKCE challenge (if any)
   ///
   /// Then backend generates an authorization code and redirects.
   OAuthAuthorizePost,
   POST "/auth/oauth/login",
   request = {
   /// The client ID of the OAuth client
   pub client_id: String,
   /// The redirect URI of the OAuth client
   pub redirect_uri: Option<String>,
   /// The scopes requested by the client
   pub scope: String,
   /// The state value originally provided by the client
   pub state: Option<String>,

   /// PKCE: code challenge sent by the client app
   pub code_challenge: String,
   /// PKCE: S256 or plain
   pub code_challenge_method: CodeChallengeHashMethod,
   },
   response_headers = {
	   /// The URL to redirect the user to
	   pub redirect_url: Location,
   }
);
