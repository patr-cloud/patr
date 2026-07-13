use std::fmt::{self, Display};

use headers::{Error, Header};
use http::{HeaderName, HeaderValue, header};

/// A lenient `Content-Range` request header.
///
/// The OCI distribution spec uses the bare `<start>-<end>` form on chunked blob
/// uploads (e.g. `0-511`), which is NOT the strict HTTP
/// `bytes <start>-<end>/<total>` form that [`headers::ContentRange`] requires —
/// parsing the OCI form with the strict type fails and the whole request is
/// rejected as "Invalid Headers". This wrapper keeps the raw value and parses
/// the start offset leniently from either form.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentRange(HeaderValue);

impl ContentRange {
	/// The starting byte offset, parsed from either the OCI `<start>-<end>`
	/// form or the HTTP `bytes <start>-<end>/<total>` form. Returns `None` if
	/// the value can't be parsed.
	pub fn start(&self) -> Option<u64> {
		let value = self.0.to_str().ok()?.trim();
		let value = value.strip_prefix("bytes ").unwrap_or(value);
		let range = value.split('/').next()?;
		let start = range.split('-').next()?;
		start.trim().parse::<u64>().ok()
	}
}

impl Header for ContentRange {
	fn name() -> &'static HeaderName {
		&header::CONTENT_RANGE
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

impl Display for ContentRange {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.0.to_str().map_err(|_| fmt::Error)?)
	}
}
