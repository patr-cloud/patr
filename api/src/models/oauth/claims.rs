use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::prelude::*;

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
