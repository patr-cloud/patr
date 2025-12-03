use std::fmt::{self, Display};

use headers::{Error, Header};
use http::{HeaderName, HeaderValue, header};

/// This struct represents a login ID.
///
/// It is used to identify a user's login in the database. A user's login can be
/// any way they access the API - Either through the website, through an API
/// request, the CLI or from an OAuth application. It is used as a header in
/// requests to the API.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Range(HeaderValue);

impl Range {
	/// Creates a new `Range` header from the given byte range.
	pub fn new(range: std::ops::Range<u64>) -> Result<Self, Error> {
		let value = format!("{}-{}", range.start, range.end - 1);
		let header_value = HeaderValue::from_str(&value).map_err(|_| Error::invalid())?;
		Ok(Self(header_value))
	}
}

impl Header for Range {
	fn name() -> &'static HeaderName {
		&header::RANGE
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
	where
		Self: Sized,
		I: Iterator<Item = &'i HeaderValue>,
	{
		let value = values.next().ok_or_else(Error::invalid)?;

		Ok(Self(value.clone()))
	}

	fn encode<E>(&self, values: &mut E)
	where
		E: Extend<HeaderValue>,
	{
		values.extend(std::iter::once(self.0.clone()));
	}
}

impl Display for Range {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.0.to_str().map_err(|_| fmt::Error)?)
	}
}
