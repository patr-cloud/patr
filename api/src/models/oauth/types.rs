use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use crate::prelude::*;

/// The identity scopes this provider understands.
///
/// These gate what `/userinfo` and the id token disclose. They deliberately
/// do **not** narrow what the issued access token can do against the API —
/// a grant acts with the user's full permissions, which is only safe while
/// every client is first-party and declared in the config.
pub const SUPPORTED_SCOPES: [&str; 4] = ["openid", "profile", "email", "offline_access"];

/// The PKCE challenge method. OAuth 2.1 removes `plain`, so `S256` is the
/// only value accepted — with `plain` the verifier *is* the challenge, sent
/// in the clear on the authorization request, which leaves PKCE protecting
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeChallengeMethod {
	/// SHA-256, base64url-encoded without padding.
	#[serde(rename = "S256")]
	S256,
}

/// A pending authorization request, parked in Redis between `/authorize`
/// validating it and the consent screen resolving it.
///
/// Deliberately carries no user id. `/authorize` is served from the API's
/// own origin and cannot read the dashboard's session cookie, so the user is
/// bound later, at consent time, from their own authenticated request. That
/// also means a leaked request id cannot be used to bind somebody else's
/// account to a grant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthAuthorizationRequest {
	/// The client that asked.
	pub client_id: String,
	/// The redirect URI, already validated against the client's registered
	/// list, so the consent endpoint can trust it.
	pub redirect_uri: String,
	/// The identity scopes requested, already checked against what the
	/// client is allowed to ask for.
	pub scopes: Vec<String>,
	/// The client's opaque `state`, echoed back untouched.
	pub state: Option<String>,
	/// The OIDC `nonce`, bound into the id token to tie it to this request.
	pub nonce: Option<String>,
	/// The PKCE challenge, verified against the verifier at `/token`.
	pub code_challenge: String,
	/// How the challenge was derived.
	pub code_challenge_method: CodeChallengeMethod,
	/// When this request was created.
	#[serde(with = "time::serde::rfc3339")]
	pub created: OffsetDateTime,
}

/// An issued authorization code, parked in Redis until `/token` exchanges it.
///
/// Everything the token endpoint needs to check is captured here at the
/// moment of issue: the code is bound to one client, one user, one redirect
/// URI and one PKCE challenge, so none of them can be swapped during the
/// exchange.
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
	/// How the challenge was derived.
	pub code_challenge_method: CodeChallengeMethod,
	/// Where consent was given from.
	pub created_ip: IpAddr,
	/// What the user consented with.
	pub created_user_agent: String,
}

/// The token pair a refresh token minted when it was consumed, parked in
/// Redis for the length of the grace window.
///
/// Clients race themselves — the CLI refreshing on two threads at once is
/// ordinary behaviour, not an attack — and without this the loser of the
/// race presents a token that is already consumed and gets the whole grant
/// revoked. Replaying the pair leaves both callers holding exactly what they
/// would have held had the requests been serialised.
///
/// The plaintext refresh token is deliberately nowhere in Postgres, so this
/// cannot be reconstructed from the row; it has to be kept. Redis is where
/// it belongs — it is wanted for [`OAUTH_REFRESH_TOKEN_GRACE_PERIOD`] and
/// never again, and a TTL expires it without a sweep.
///
/// [`OAUTH_REFRESH_TOKEN_GRACE_PERIOD`]: crate::utils::constants::OAUTH_REFRESH_TOKEN_GRACE_PERIOD
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthReplacementTokens {
	/// The access token that was minted.
	pub access_token: String,
	/// The refresh token that was minted, in plaintext.
	pub refresh_token: String,
}

/// Hashes a code for use as its Redis key.
///
/// Keyed on the hash rather than the code itself so that a Redis dump, or a
/// stray `KEYS *`, does not hand out live authorization codes — the same
/// reasoning that keeps tokens hashed in Postgres.
pub fn hash_code(code: &str) -> String {
	hex::encode(Sha256::digest(code.as_bytes()))
}
