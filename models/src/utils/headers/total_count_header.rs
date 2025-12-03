use headers::{Error, Header};
use http::{HeaderName, HeaderValue};

/// This struct represents the total count of items that are available for the
/// query. This is used to set the `X-Total-Count` header in the response.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct TotalCountHeader(pub usize);

/// A header that is added to the response to indicate the total number of
/// items that are available for the query (usually for list routes).
static HEADER_NAME: HeaderName = HeaderName::from_static("x-total-count");

impl Header for TotalCountHeader {
	fn name() -> &'static HeaderName {
		&HEADER_NAME
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
	where
		Self: Sized,
		I: Iterator<Item = &'i HeaderValue>,
	{
		let value = values.next().ok_or_else(headers::Error::invalid)?;

		let count = value
			.to_str()
			.map_err(|_| headers::Error::invalid())?
			.parse::<usize>()
			.map_err(|_| headers::Error::invalid())?;

		Ok(Self(count))
	}

	fn encode<E>(&self, values: &mut E)
	where
		E: Extend<HeaderValue>,
	{
		values.extend(std::iter::once(
			HeaderValue::from_str(&self.0.to_string()).expect("HeaderValue should be valid UTF-8"),
		));
	}
}
