use headers::{Error, Header};
use http::{HeaderName, HeaderValue};

/// A wrapper struct that represents an optional header. It is used to
/// indicate that a header may or may not be present in the request/response.
/// This is useful for headers that are not mandatory.
///  Example: An optional `X-Custom-Header` header.
#[derive(Debug, Clone, PartialEq)]
pub struct OptionalHeader<T>(Option<T>)
where
	T: Header;

impl<T> OptionalHeader<T>
where
	T: Header,
{
	/// Creates a new OptionalHeader with the given value.
	pub fn new(value: Option<T>) -> Self {
		Self(value)
	}

	/// Consumes the wrapper and returns the inner value, if present.
	pub fn into_option(self) -> Option<T> {
		self.0
	}
}

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
		if values.size_hint() == (0, Some(0)) {
			Ok(Self(None))
		} else {
			Ok(Self(Some(T::decode(values)?)))
		}
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

#[cfg(test)]
mod tests {
	use headers::{HeaderMapExt, HeaderValue, Range};
	use http::HeaderMap;

	use super::*;

	#[test]
	fn test_optional_header_decode_present() {
		let headers = vec![(
			HeaderName::from_static("range"),
			HeaderValue::from_static("bytes=0-1024"),
		)]
		.into_iter()
		.collect::<HeaderMap>();
		let optional_header = headers.typed_get::<OptionalHeader<Range>>().unwrap();
		assert!(optional_header.into_option().is_some());
	}

	#[test]
	fn test_optional_header_decode_absent() {
		let headers = vec![].into_iter().collect::<HeaderMap>();
		let optional_header = <OptionalHeader<Range> as ::headers::Header>::decode(
			&mut (headers
				.get_all(<OptionalHeader<Range> as ::headers::Header>::name())
				.iter()),
		)
		.unwrap();
		assert_eq!(optional_header.into_option(), None);
	}
}
