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

		if !value
			.to_str()
			.map(|value| value.starts_with(Bearer::SCHEME))
			.unwrap_or(false)
		{
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
