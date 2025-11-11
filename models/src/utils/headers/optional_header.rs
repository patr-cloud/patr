use headers::{Error, Header};
use http::{HeaderName, HeaderValue};

/// A wrapper struct that represents an optional header. It is used to
/// indicate that a header may or may not be present in the request/response.
/// This is useful for headers that are not mandatory.
///  Example: An optional `X-Custom-Header` header.
#[derive(Debug, Clone, PartialEq)]
pub struct OptionalHeader<T>(pub Option<T>)
where
	T: Header;

impl<T> Header for OptionalHeader<T>
where
	T: Header,
{
	fn name() -> &'static HeaderName {
		T::name()
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
	where
		Self: Sized,
		I: Iterator<Item = &'i HeaderValue>,
	{
		let value = match T::decode(values) {
			Ok(v) => Some(v),
			Err(_) => None,
		};

		Ok(Self(value))
	}

	fn encode<E>(&self, values: &mut E)
	where
		E: Extend<HeaderValue>,
	{
		if let Some(inner) = &self.0 {
			inner.encode(values);
		}
	}
}
