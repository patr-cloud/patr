use headers::{Error, Header};
use http::{HeaderName, HeaderValue};

/// Custom header for Docker-Content-Digest
#[derive(Debug, Clone, PartialEq)]
pub struct DockerContentDigest(pub String);

/// The header name for Docker-Content-Digest
static HEADER_NAME: HeaderName = HeaderName::from_static("docker-content-digest");

impl Header for DockerContentDigest {
	fn name() -> &'static HeaderName {
		&HEADER_NAME
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
