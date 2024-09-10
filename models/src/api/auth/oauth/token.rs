use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// The grant type for the request
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OAuthTokenGrantType {
	/// The request is for a temporary authorization code that will be exchanged
	/// for an access token and a refresh token.
	AuthorizationCode,
	/// The request is for a new access token using a refresh token.
	RefreshToken,
}

macros::declare_api_endpoint!(
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
	OAuthToken,
	POST "/auth/oauth/token",
	request = {
		/// The grant type for the request
		#[serde(rename = "grant_type")]
		pub grant_type: OAuthTokenGrantType,
		/// The client ID of the third-party app
		#[serde(rename = "client_id")]
		pub client_id: String,
		/// The redirect URI of the third-party app
		#[serde(rename = "redirect_uri", default, skip_serializing_if = "Option::is_none")]
		pub redirect_uri: Option<String>,
		/// The authorization code received from the `/authorize` endpoint
		pub code: String,
		/// The code verifier used to hash the code challenge
		#[serde(rename = "code_verifier")]
		pub code_verifier: String,
		/// The refresh token (required for refresh_token grant type)
		#[serde(rename = "refresh_token", default, skip_serializing_if = "Option::is_none")]
		pub refresh_token: Option<String>,
	},
	response = {
		/// The access token that the third-party app can use to access the user's
		/// data.
		#[serde(rename = "access_token")]
		pub access_token: String,
		/// The type of token that was issued
		#[serde(rename = "token_type")]
		pub token_type: String,
		/// The time in seconds that the access token is valid for
		#[serde(rename = "expires_in")]
		pub expires_in: usize,
		/// The refresh token that the third-party app can use to get a new access
		/// token.
		#[serde(rename = "refresh_token")]
		pub refresh_token: String,
		/// The scopes that the access token has access to
		pub scope: String
	}
);
