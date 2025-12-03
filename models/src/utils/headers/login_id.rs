use headers::{Error, Header};
use http::{HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};

use crate::prelude::Uuid;

/// This struct represents a login ID.
///
/// It is used to identify a user's login in the database. A user's login can be
/// any way they access the API - Either through the website, through an API
/// request, the CLI or from an OAuth application. It is used as a header in
/// requests to the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LoginId(pub Uuid);

/// The header name used for the [`LoginId`] header.
static HEADER_NAME: HeaderName = HeaderName::from_static("x-login-id");

impl Header for LoginId {
	fn name() -> &'static HeaderName {
		&HEADER_NAME
	}

	fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
	where
		Self: Sized,
		I: Iterator<Item = &'i HeaderValue>,
	{
		let value = values.next().ok_or_else(headers::Error::invalid)?;

		let uuid = value
			.to_str()
			.map_err(|_| headers::Error::invalid())
			.map(Uuid::parse_str)
			.map_err(|_| headers::Error::invalid())?
			.map_err(|_| headers::Error::invalid())?;

		Ok(Self(uuid))
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
