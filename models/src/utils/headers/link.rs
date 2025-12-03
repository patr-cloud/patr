use headers::{Error, Header};
use http::{HeaderName, HeaderValue};

/// Custom header for Link (pagination)
#[derive(Debug, Clone, PartialEq)]
pub struct Link(String);

impl Link {
	/// Create a new Link header with the given link value
	pub fn new(link: impl Into<String>) -> Self {
		Self(link.into())
	}
}

static NAME: HeaderName = HeaderName::from_static("link");

impl Header for Link {
	fn name() -> &'static HeaderName {
		&NAME
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
