use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::prelude::*;

/// The identity scopes this provider issues. They gate what `/userinfo` and
/// the id token disclose, not what the access token can do.
pub const SUPPORTED_SCOPES: [&str; 4] = ["openid", "profile", "email", "offline_access"];

/// A pending authorization request, parked in Redis between `/authorize` and
/// the consent screen. It has no user: the consent endpoint binds the caller.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthAuthorizationRequest {
	/// The client that asked.
	pub client_id: String,
	/// Already matched against the client's registered list.
	pub redirect_uri: String,
	/// Already checked against what the client may request.
	pub scopes: Vec<String>,
	/// The client's opaque `state`, echoed back untouched.
	pub state: Option<String>,
	/// The OIDC `nonce`, bound into the id token.
	pub nonce: Option<String>,
	/// The PKCE challenge, verified against the verifier at `/token`.
	pub code_challenge: String,
}

/// An issued authorization code, parked in Redis until `/token` exchanges it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthAuthorizationCode {
	/// The client this code was issued to.
	pub client_id: String,
	/// The user who approved it.
	pub user_id: Uuid,
	/// The web login that approved it.
	pub approving_login_id: Uuid,
	/// When that login authenticated, for the id token's `auth_time`.
	#[serde(with = "time::serde::rfc3339")]
	pub auth_time: OffsetDateTime,
	/// The redirect URI, which `/token` requires to match again.
	pub redirect_uri: String,
	/// The approved identity scopes.
	pub scopes: Vec<String>,
	/// The OIDC `nonce`, if the client sent one.
	pub nonce: Option<String>,
	/// The PKCE challenge to verify the verifier against.
	pub code_challenge: String,
	/// Where consent was given from.
	pub created_ip: IpAddr,
	/// What the user consented with.
	pub created_user_agent: String,
}

/// The pair a refresh token minted when it was consumed, kept in Redis for
/// the grace window so a client racing itself gets the same pair back.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthReplacementTokens {
	/// The access token that was minted.
	pub access_token: String,
	/// The refresh token that was minted.
	pub refresh_token: String,
}
