use std::str::FromStr;

use headers::{
	Authorization,
	Error,
	Header,
	authorization::{Bearer, Credentials as _},
};
use http::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};

/// This struct represents a bearer token. It is used to authenticate a user's
/// request to the API. It is used as a header in requests to the API.
///
/// This is a wrapper around [`Bearer`].
/// Example: Authorization: Bearer *token*
#[derive(Debug, Clone, PartialEq)]
pub struct BearerToken(pub Bearer);

impl FromStr for BearerToken {
	type Err = headers::Error;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		Ok(Self(
			Bearer::decode(
				&HeaderValue::from_str(&format!("Bearer {value}"))
					.map_err(|_| headers::Error::invalid())?,
			)
			.ok_or_else(headers::Error::invalid)?,
		))
	}
}

impl Header for BearerToken {
	fn name() -> &'static HeaderName {
		Authorization::<Bearer>::name()
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
	where
		Self: Sized,
		I: Iterator<Item = &'i HeaderValue>,
	{
		let value = values.next().ok_or_else(Error::invalid)?;

		let has_valid_token = value
			.to_str()
			.ok()
			.and_then(|value| value.strip_prefix("Bearer "))
			.is_some_and(|token| !token.trim().is_empty());
		if !has_valid_token {
			return Err(Error::invalid());
		}

		let value = Bearer::decode(value).ok_or_else(Error::invalid)?;

		Ok(Self(value))
	}

	fn encode<E>(&self, values: &mut E)
	where
		E: Extend<HeaderValue>,
	{
		values.extend(std::iter::once(self.0.encode()));
	}
}

impl Serialize for BearerToken {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(self.0.token())
	}
}

impl<'de> Deserialize<'de> for BearerToken {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		Authorization::bearer(&String::deserialize(deserializer)?)
			.map_err(serde::de::Error::custom)
			.map(|Authorization(val)| val)
			.map(Self)
	}
}

#[cfg(test)]
mod tests {
	use headers::Header;
	use http::HeaderValue;

	use super::BearerToken;

	fn decode(value: &str) -> Result<BearerToken, headers::Error> {
		let value = HeaderValue::from_str(value).unwrap();
		BearerToken::decode(&mut std::iter::once(&value))
	}

	#[test]
	fn rejects_malformed_bearer_without_panicking() {
		// A bare "Bearer" (6 bytes) is the value that used to panic the process:
		// `Bearer::token()` slices `[7..]` on it. These must all be a clean error.
		assert!(decode("Bearer").is_err());
		assert!(decode("Bearer ").is_err());
		assert!(decode("Bearer    ").is_err());
		assert!(decode("Basic abc123").is_err());
		assert!(decode("patrv1.abc.def").is_err());
	}

	#[test]
	fn accepts_valid_bearer_and_reads_token() {
		let BearerToken(bearer) = decode("Bearer patrv1.abc.def").unwrap();
		// `.token()` is the method that panics on a malformed value; a valid
		// token round-trips without stripping into out-of-bounds territory.
		assert_eq!(bearer.token(), "patrv1.abc.def");
	}
}
