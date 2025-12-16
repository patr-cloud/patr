use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// The type of request that the third-party app is making.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum OAuthAuthorizeResponseType {
	/// The third-party app is requesting a temporary authorization code.
	#[serde(rename = "code")]
	#[default]
	AuthorizationCode,
}

/// The method used to hash the code challenge.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum CodeChallengeHashMethod {
	/// The code challenge is hashed using the SHA-256 algorithm.
	#[serde(rename = "S256")]
	SHA256,
	/// The code challenge is not hashed (plain text).
	#[serde(rename = "plain")]
	Plain,
}

impl std::fmt::Display for CodeChallengeHashMethod {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			CodeChallengeHashMethod::SHA256 => write!(f, "S256"),
			CodeChallengeHashMethod::Plain => write!(f, "plain"),
		}
	}
}

macros::declare_api_endpoint!(
	/// The endpoint to authorize a user.
	///
	/// This is the first step. The third-party app opens a browser and sends the
	/// user to this endpoint on our API. Here, the user logs in (using the
	/// frontend) and gives the third-party app permission to access their data in
	/// the API. This endpoint is used to authorize a user using OAuth2.1. It
	/// returns a temporary code that can be exchanged for an access token and a
	/// refresh token.
	OAuthAuthorize,
	GET "/auth/oauth/authorize",
	query = {
		/// The request type that the third-party app is making
		pub response_type: OAuthAuthorizeResponseType,
		/// The client ID of the third-party app
		pub client_id: String,
		/// The secret associatded with the client
		pub client_secret: String,
		/// The redirect URI of the third-party app
		#[serde(default, skip_serializing_if = "Option::is_none")]
		pub redirect_uri: Option<String>,
		/// The scopes requested by the third-party app
		pub scope: String,
		/// The state of the request, if any
		pub state: Option<String>,
		/// The hashed value of a code challenge (as per PKCE)
		pub code_challenge: String,
		/// The method used to hash the code challenge
		pub code_challenge_method: CodeChallengeHashMethod,
	},
	response_headers = {
		/// The URL to redirect the user to
		pub redirect_url: Location,
	}
);
