use axum::{body::Body, http::StatusCode};
use typed_builder::TypedBuilder;

use crate::routes::registry_patr_cloud::prelude::*;

/// Response object for registry endpoints.
///
/// This struct provides helpers for creating streaming responses with
/// appropriate headers and status codes, following the OCI Distribution
/// Specification.
#[derive(Debug, TypedBuilder)]
pub struct RegistryResponse<E>
where
	E: RegistryEndpoint,
{
	/// HTTP status code (default: 200 OK)
	pub status_code: StatusCode,
	/// Response headers (e.g., Content-Type, Docker-Content-Digest)
	pub headers: E::ResponseHeaders,
	/// Streaming response body (not buffered in memory)
	pub body: Body,
}

impl<E> RegistryResponse<E>
where
	E: RegistryEndpoint,
{
	/// Convert the response into a Result
	///
	/// # Errors
	/// This function will always return Ok(self) as the response is always
	/// successful. This is used to convert the response into a Result type
	/// for ease of use.
	pub fn into_result(self) -> Result<Self, RegistryError> {
		Ok(self)
	}
}
