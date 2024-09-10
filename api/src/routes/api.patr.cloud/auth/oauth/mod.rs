use axum::Router;

use crate::prelude::*;

/// The endpoint to authorize a user.
///
/// This is the first step. The third-party app opens a browser and sends the
/// user to this endpoint on our API. Here, the user logs in (using the
/// frontend) and gives the third-party app permission to access their data in
/// the API. This endpoint is used to authorize a user using OAuth2.1. It
/// returns a temporary code that can be exchanged for an access token and a
/// refresh token.
mod authorize;
/// The endpoint to get details about the access token and refresh token.
///
/// The third-party app can call this endpoint to get information about the
/// access token and refresh token. This is useful for debugging and monitoring
/// the tokens. It’s like asking, "Is this key still good?" This endpoint is
/// used to get details about the access token and refresh token. It returns
/// information about the token, such as the user ID, the scopes, and the expiry
/// time.
mod introspect;
/// The endpoint to revoke an access token.
///
/// If the user or the third-party app wants to stop the app’s access (like
/// logging out or revoking permission), the app can call this endpoint to tell
/// the API to deactivate the access token and refresh token. This endpoint is
/// used to revoke an access token. This is useful when a user wants to log out
/// of a third-party app. The access token is invalidated and the user will have
/// to log in again to get a new access token.
mod revoke;
/// The endpoint to exchange a temporary code for an access token and a refresh
/// token.
///
/// After the user approves the third-party app, the API  server gives the app a
/// temporary code. The app sends this code to the /token endpoint to exchange
/// it for an access token and a refresh token. The access token is what the
/// third-party app will use to access the user’s data on the API. This endpoint
/// is used to exchange a temporary code for an access token and a
/// refresh token. The temporary code is obtained by authorizing a user using
/// the [`authorize`] endpoint.
mod token;

use self::{authorize::*, introspect::*, revoke::*, token::*};

/// Sets up the oauth routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_endpoint(authorize, state)
		.mount_endpoint(introspect, state)
		.mount_endpoint(revoke, state)
		.mount_endpoint(token, state)
}
