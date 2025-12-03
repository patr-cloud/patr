use std::fmt::Display;

use headers::{Error, Header};
use http::{HeaderName, HeaderValue};

use crate::prelude::*;

/// Custom header for Docker upload UUID
#[derive(Debug, Clone, PartialEq)]
pub struct DockerUploadUuid(Uuid);

impl DockerUploadUuid {
	/// Creates a new DockerUploadUuid header from a Uuid.
	pub fn new(uuid: Uuid) -> Self {
		Self(uuid)
	}
}

impl Display for DockerUploadUuid {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

static HEADER_NAME: HeaderName = HeaderName::from_static("docker-upload-uuid");

impl Header for DockerUploadUuid {
	fn name() -> &'static HeaderName {
		&HEADER_NAME
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
	where
		I: Iterator<Item = &'i HeaderValue>,
	{
		let value = values.next().ok_or_else(Error::invalid)?;
		let uuid = value
			.to_str()
			.map_err(|_| Error::invalid())?
			.parse()
			.map_err(|_| Error::invalid())?;
		Ok(Self(uuid))
	}

	fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
		if let Ok(value) = HeaderValue::from_str(&self.0.to_string()) {
			values.extend(std::iter::once(value));
		}
	}
}
