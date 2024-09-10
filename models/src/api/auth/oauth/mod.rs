//!
//! Lessons learned from OAuth2.1 spec research:
//! - **Client Credentials** flow: `client_credentials` grant type is for
//!   machine-to-machine communication. This is when a backend server wants to
//!   communicate with our API and has no user interaction whatsoever. Just
//!   wants to communicate with our API directly from their backend. They are
//!   given a client ID and a client secret. The 3rd part app (client) sends
//!   these to the API server to get an access token. It's pretty much like a
//!   username and password.
//! - **Authorization Code** flow: First, the `code` grant type is used to tell
//!   the API server that the client wants to exchange a temporary code for an
//!   access token and a refresh token. Along with this request, it sends a
//!   code_challenge (a hash of a random set of characters) and a
//!   code_challenge_method (the method used to hash the code_challenge). The
//!   API server just happily carries this code_challenge along for all the next
//!   set of steps (till the callback URL). The client sends the user to the API
//!   server to authorize the client. The user logs in and gives the client
//!   permission to access their data. The API server then redirects to a
//!   specific callback URL (that has to be an exact match of something that was
//!   pre-registered with the client) along with a temporary code and the
//!   code_challenge. The client sends this code to the API server (with the
//!   `authorization_code` grant) along with the original un-hashed
//!   code_challenge to exchange it for an access token and a refresh token. The
//!   API server verifies that the un-hashed code_challenge and the hashed value
//!   that was originally provided match. The client can then use the access
//!   token to access the user's data.

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

pub use self::{authorize::*, introspect::*, revoke::*, token::*};
