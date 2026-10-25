use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{models::oauth::identity::IdentityClaims, prelude::*};

/// The claims in an OAuth access token. Not [`AccessTokenData`][1], whose
/// `camelCase` would mangle the names OIDC defines.
///
/// [1]: crate::models::access_token_data::AccessTokenData
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAccessTokenClaims {
	/// The issuer.
	pub iss: String,
	/// The user.
	pub sub: Uuid,
	/// The API. An id token's audience is the client, so checking this keeps
	/// one from being used as an API credential.
	pub aud: String,
	/// The client acting on the user's behalf.
	pub azp: String,
	/// The grant: its `oauth_login` row.
	pub sid: Uuid,
	/// The granted identity scopes, space-delimited.
	pub scope: String,
	/// When this token expires.
	#[serde(with = "crate::models::access_token_data::datetime_as_seconds")]
	pub exp: OffsetDateTime,
	/// Not valid before this.
	#[serde(with = "crate::models::access_token_data::datetime_as_seconds")]
	pub nbf: OffsetDateTime,
	/// When it was issued.
	#[serde(with = "crate::models::access_token_data::datetime_as_seconds")]
	pub iat: OffsetDateTime,
	/// A unique id for this token.
	pub jti: Uuid,
}

/// The claims in an OIDC id token: a statement about a login, for the client
/// that asked for it, and not a credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdTokenClaims {
	/// The issuer.
	pub iss: String,
	/// The client. An access token's audience is the API instead.
	pub aud: String,
	/// When this token expires.
	#[serde(with = "crate::models::access_token_data::datetime_as_seconds")]
	pub exp: OffsetDateTime,
	/// When it was issued.
	#[serde(with = "crate::models::access_token_data::datetime_as_seconds")]
	pub iat: OffsetDateTime,
	/// When the web login that approved the grant authenticated.
	#[serde(with = "crate::models::access_token_data::datetime_as_seconds")]
	pub auth_time: OffsetDateTime,
	/// The client's `nonce`, echoed back.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub nonce: Option<String>,
	/// The left half of the access token's SHA-256, base64url-encoded (OIDC
	/// Core section 3.1.3.6).
	pub at_hash: String,
	/// `sub` and whatever the scopes allow, the same as `/userinfo` returns.
	#[serde(flatten)]
	pub identity: IdentityClaims,
}
