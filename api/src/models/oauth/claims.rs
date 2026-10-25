use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::prelude::*;

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
