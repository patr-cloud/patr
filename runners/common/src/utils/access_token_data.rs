use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::prelude::*;

/// A struct representing the data that is stored inside the access token, which
/// will be encoded as a JWT. Remember, JWTs can be decoded on the client side,
/// so no sensitive data should be stored here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenData {
	/// RFC7519:
	/// The "iss" (issuer) claim identifies the principal that issued the JWT.
	///
	/// I believe this means that the issuer is the server that issued the token
	/// (in this case, The Users Server)?
	pub iss: String,
	/// RFC7519:
	/// The "sub" (subject) claim identifies the principal that is the subject
	/// of the JWT.  The claims in a JWT are normally statements about the
	/// subject.  The subject value MUST either be scoped to be locally unique
	/// in the context of the issuer or be globally unique.
	///
	/// I think this is the user's login ID (In this case (self-hosted PATR),
	/// since there is no "user", it's an empty string, I'm still keeping it
	/// here to comply with the spec, but don't use it in this context, as we
	/// might decide to remove it later)
	pub sub: String,
	/// RFC7519:
	/// The "aud" (audience) claim identifies the recipients that the JWT is
	/// intended for.  Each principal intended to process the JWT MUST identify
	/// itself with a value in the audience claim.  If the principal processing
	/// the claim does not identify itself with a value in the "aud" claim when
	/// this claim is present, then the JWT MUST be rejected.  In the general
	/// case, the "aud" value is an array of case-sensitive strings, each
	/// containing a StringOrURI value.  In the special case when the JWT has
	/// one audience, the "aud" value MAY be a single case-sensitive string
	/// containing a StringOrURI value.
	///
	/// I'm guessing this is the list of clients that are allowed to process
	/// this JWT. Since this is the self hosted version of patr, the audience
	/// can be, well anything? I'm not checking against this, but still keeping
	/// it around if we ever decide to use it somewhere.
	pub aud: OneOrMore<String>,
	/// RFC7519:
	/// The "exp" (expiration time) claim identifies the expiration time on or
	/// after which the JWT MUST NOT be accepted for processing.  The
	/// processing of the "exp" claim requires that the current date/time MUST
	/// be before the expiration date/time listed in the "exp" claim.
	/// Implementers MAY provide for some small leeway, usually no more than a
	/// few minutes, to account for clock skew.
	///
	/// This is the timestamp (in seconds) when the JWT has expired.
	#[serde(with = "datetime_as_seconds")]
	pub exp: OffsetDateTime,
	/// RFC7519:
	/// The "nbf" (not before) claim identifies the time before which the JWT
	/// MUST NOT be accepted for processing.  The processing of the "nbf" claim
	/// requires that the current date/time MUST be after or equal to the
	/// not-before date/time listed in the "nbf" claim.  Implementers MAY
	/// provide for some small leeway, usually no more than a few minutes, to
	/// account for clock skew.
	///
	/// Any JWT received before this timestamp (in seconds) should be rejected.
	#[serde(with = "datetime_as_seconds")]
	pub nbf: OffsetDateTime,
	/// RFC7519:
	/// The "iat" (issued at) claim identifies the time at which the JWT was
	/// issued.  This claim can be used to determine the age of the JWT.
	///
	/// This is just a timestamp of when the JWT was created.
	#[serde(with = "datetime_as_seconds")]
	pub iat: OffsetDateTime,
	/// RFC7519:
	/// The "jti" (JWT ID) claim provides a unique identifier for the JWT. The
	/// identifier value MUST be assigned in a manner that ensures that there is
	/// a negligible probability that the same value will be accidentally
	/// assigned to a different data object; if the application uses multiple
	/// issuers, collisions MUST be prevented among values produced by different
	/// issuers as well.  The "jti" claim can be used to prevent the JWT from
	/// being replayed.
	///
	/// This can perhaps be a UUID v1, and any JWT with an old jti can be
	/// rejected.
	pub jti: Uuid,
}

impl AccessTokenData {
	/// The validity of a refresh token. After this time, the refresh token will
	/// be considered expired, and the client should handle this by logging out
	/// the user, or attempting to login again.
	pub const REFRESH_TOKEN_VALIDITY: Duration = Duration::days(30);
}

/// A module to help serialize and deserialize `OffsetDateTime` as seconds
mod datetime_as_seconds {
	use serde::{de::Error, Deserialize, Deserializer, Serializer};
	use time::OffsetDateTime;

	/// Serialize an `OffsetDateTime` as seconds
	pub fn serialize<S>(value: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_i64(value.unix_timestamp())
	}

	/// Deserialize an `OffsetDateTime` from seconds
	pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
	where
		D: Deserializer<'de>,
	{
		OffsetDateTime::from_unix_timestamp(i64::deserialize(deserializer)?).map_err(Error::custom)
	}
}
