use headers::{Error, Header};
use http::{HeaderName, HeaderValue};

/// Custom header for Docker-Distribution-API-Version
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DockerDistributionApiVersion;

static HEADER_NAME: HeaderName = HeaderName::from_static("docker-distribution-api-version");

impl Header for DockerDistributionApiVersion {
	fn name() -> &'static HeaderName {
		&HEADER_NAME
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
	where
		I: Iterator<Item = &'i HeaderValue>,
	{
		let value = values.next().ok_or_else(Error::invalid)?;
		let str_value = value.to_str().map_err(|_| Error::invalid())?;

		// this should only be "registry/2.0"

		if str_value != "registry/2.0" {
			return Err(Error::invalid());
		}

		Ok(Self)
	}

	fn encode<E>(&self, values: &mut E)
	where
		E: Extend<HeaderValue>,
	{
		let value = HeaderValue::from_static("registry/2.0");
		values.extend(std::iter::once(value));
	}
}
