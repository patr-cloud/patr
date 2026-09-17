use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{models::oauth::identity::IdentityClaims, prelude::*};

/// The claims in an OAuth access token.
///
/// Deliberately not [`AccessTokenData`][1], which is the web dashboard's:
/// that one is `camelCase`, which would mangle the snake_case names OIDC
/// defines, and its `sub` means something else. Keeping them apart also
/// keeps the two token families from being interchangeable by accident.
///
/// [1]: crate::models::access_token_data::AccessTokenData
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAccessTokenClaims {
	/// The issuer, which must match the one published in the discovery
	/// document.
	pub iss: String,
	/// The **user**, stable across every grant they hold. Not the grant —
	/// that is [`Self::sid`].
	pub sub: Uuid,
	/// The API this token may be presented to.
	///
	/// Load-bearing. An id token names the *client* as its audience, and
	/// relying parties treat id tokens as non-secret — they end up in logs
	/// and session cookies. Pinning this on the way in is what stops one
	/// being replayed here as an API credential.
	pub aud: String,
	/// The client acting on the user's behalf. Informational: the API does
	/// not vary its behaviour by client.
	pub azp: String,
	/// The grant this token was issued under — the `oauth_login` row the
	/// authenticator looks up to resolve permissions and check revocation.
	pub sid: Uuid,
	/// The granted identity scopes, space-delimited. Informational here;
	/// they gate `/userinfo` and the id token, not API authority.
	pub scope: String,
	/// When this token expires.
	#[serde(with = "datetime_as_seconds")]
	pub exp: OffsetDateTime,
	/// Not valid before this.
	#[serde(with = "datetime_as_seconds")]
	pub nbf: OffsetDateTime,
	/// When it was issued.
	#[serde(with = "datetime_as_seconds")]
	pub iat: OffsetDateTime,
	/// A unique id for this token.
	pub jti: Uuid,
}

/// The claims in an OIDC id token.
///
/// Not a credential: an id token is a signed statement *about* a login, for
/// the client that asked for it, and relying parties treat it as non-secret.
/// Which is why its audience is the client rather than the API, and why the
/// authenticator refuses one presented as a bearer token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdTokenClaims {
	/// The issuer, which the client checks against its configuration.
	pub iss: String,
	/// The client this token was minted for. An access token names the API
	/// instead; keeping the two apart is what makes them non-interchangeable.
	pub aud: String,
	/// When this token expires.
	#[serde(with = "datetime_as_seconds")]
	pub exp: OffsetDateTime,
	/// When it was issued.
	#[serde(with = "datetime_as_seconds")]
	pub iat: OffsetDateTime,
	/// When the user actually authenticated in the browser session that
	/// approved the grant — which can be a good deal earlier than `iat`.
	#[serde(with = "datetime_as_seconds")]
	pub auth_time: OffsetDateTime,
	/// The value the client sent on the authorization request, echoed back
	/// verbatim. The client compares it to what it stored, which is what ties
	/// this token to the request it started rather than one an attacker
	/// injected.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub nonce: Option<String>,
	/// The left-most 128 bits of `SHA-256(access_token)`, base64url-encoded.
	///
	/// Lets a client confirm the access token it received belongs with this
	/// id token, so the two cannot be mixed from different responses.
	pub at_hash: String,
	/// `sub` and the scope-gated profile claims, flattened in so this reads
	/// as one object on the wire and cannot drift from `/userinfo` — OIDC
	/// Core section 5.3.2 requires the two agree.
	#[serde(flatten)]
	pub identity: IdentityClaims,
}

/// Serialises an `OffsetDateTime` as the unix seconds JWTs use.
mod datetime_as_seconds {
	use serde::{Deserialize, Deserializer, Serializer, de::Error};
	use time::OffsetDateTime;

	/// Writes the timestamp as seconds since the epoch.
	pub fn serialize<S>(value: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_i64(value.unix_timestamp())
	}

	/// Reads a timestamp from seconds since the epoch.
	pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
	where
		D: Deserializer<'de>,
	{
		OffsetDateTime::from_unix_timestamp(i64::deserialize(deserializer)?).map_err(Error::custom)
	}
}
