use std::{fmt::Display, str::FromStr};

use headers::{Error, Header};
use http::{HeaderName, HeaderValue, header};

/// Custom header for Location
#[derive(Debug, Clone, PartialEq)]
pub struct Location(String);

impl FromStr for Location {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if HeaderValue::from_str(value).is_err() {
			return Err(Error::invalid());
		}
		Ok(Self(value.to_string()))
	}
}

impl Display for Location {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl Header for Location {
	fn name() -> &'static HeaderName {
		&header::LOCATION
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
	where
		I: Iterator<Item = &'i HeaderValue>,
	{
		let value = values.next().ok_or_else(Error::invalid)?;
		let str_value = value.to_str().map_err(|_| Error::invalid())?;
		Ok(Self(str_value.to_string()))
	}

	fn encode<E>(&self, values: &mut E)
	where
		E: Extend<HeaderValue>,
	{
		if let Ok(value) = HeaderValue::from_str(&self.0) {
			values.extend(std::iter::once(value));
		}
	}
}
