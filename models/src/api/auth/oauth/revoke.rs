use crate::prelude::*;

macros::declare_api_endpoint!(
	/// The endpoint to revoke an access token.
	///
	/// If the user or the third-party app wants to stop the app’s access (like
	/// logging out or revoking permission), the app can call this endpoint to tell
	/// the API to deactivate the access token and refresh token. This endpoint is
	/// used to revoke an access token. This is useful when a user wants to log out
	/// of a third-party app. The access token is invalidated and the user will have
	/// to log in again to get a new access token.
	OAuthRevokeToken,
	POST "/auth/oauth/revoke",
	request = {
		/// The access token to revoke
		#[serde(rename = "access_token")]
		pub token: String,
	}
);
