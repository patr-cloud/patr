//! Registry error types.
//!
//! This module provides error handling for the OCI registry,
//! wrapping oci-spec error codes and providing conversions to HTTP responses.

use std::{error::Error as StdError, fmt};

use aws_sdk_s3::{error::SdkError, presigning::PresigningConfigError, primitives::ByteStreamError};
use axum::{
	Json,
	http::StatusCode,
	response::{IntoResponse, Response},
};
use oci_spec::distribution::{ErrorCode, ErrorInfoBuilder, ErrorResponseBuilder};
use typed_builder::TypedBuilder;

use crate::routes::registry_patr_cloud::prelude::*;

/// Registry error type that wraps OCI Distribution error codes.
///
/// This type provides a bridge between Rust errors and OCI-compliant error
/// responses, ensuring all errors are properly formatted according to the OCI
/// Distribution Specification.
#[derive(Debug, TypedBuilder)]
pub struct RegistryError {
	/// The OCI error code
	code: ErrorCode,
	/// Human-readable error message
	#[builder(setter(into))]
	message: String,
	/// Optional detailed error information
	#[builder(default, setter(strip_option))]
	detail: Option<String>,
	/// HTTP status code to return
	status: StatusCode,
}

impl RegistryError {
	/// Create a new registry error with the given code and message.
	///
	/// # Arguments
	///
	/// * `code` - The OCI error code
	/// * `message` - Human-readable error message
	pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
		let status = Self::error_code_to_status(&code);
		Self {
			code,
			message: message.into(),
			detail: None,
			status,
		}
	}

	/// Map OCI error codes to HTTP status codes.
	pub fn error_code_to_status(code: &ErrorCode) -> StatusCode {
		match code {
			ErrorCode::BlobUnknown => StatusCode::NOT_FOUND,
			ErrorCode::BlobUploadInvalid => StatusCode::BAD_REQUEST,
			ErrorCode::BlobUploadUnknown => StatusCode::NOT_FOUND,
			ErrorCode::DigestInvalid => StatusCode::BAD_REQUEST,
			ErrorCode::ManifestBlobUnknown => StatusCode::NOT_FOUND,
			ErrorCode::ManifestInvalid => StatusCode::BAD_REQUEST,
			ErrorCode::ManifestUnknown => StatusCode::NOT_FOUND,
			ErrorCode::NameInvalid => StatusCode::BAD_REQUEST,
			ErrorCode::NameUnknown => StatusCode::NOT_FOUND,
			ErrorCode::SizeInvalid => StatusCode::BAD_REQUEST,
			ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
			ErrorCode::Denied => StatusCode::FORBIDDEN,
			ErrorCode::Unsupported => StatusCode::BAD_REQUEST,
			ErrorCode::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
		}
	}

	/// Convert self into a Result that is always Err(self).
	/// This is a convenience method for use in functions that return Result.
	pub fn into_result<T>(self) -> Result<T, Self> {
		Err(self)
	}
}

impl fmt::Display for RegistryError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{:?}: {}", self.code, self.message)
	}
}

// Implement From<E> where E: std::error::Error for common error types
impl From<sqlx::Error> for RegistryError {
	fn from(err: sqlx::Error) -> Self {
		error!("Database error: {}", err);
		// Use Unsupported as a generic internal error code
		Self::builder()
			.code(ErrorCode::Unsupported)
			.message("internal server error: database operation failed")
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.build()
	}
}

impl From<serde_json::Error> for RegistryError {
	fn from(err: serde_json::Error) -> Self {
		error!("JSON error: {}", err);
		Self::new(ErrorCode::ManifestInvalid, format!("invalid JSON: {}", err))
	}
}

impl From<oci_spec::OciSpecError> for RegistryError {
	fn from(err: oci_spec::OciSpecError) -> Self {
		error!("OCI spec error: {}", err);
		Self::new(
			ErrorCode::Unsupported,
			format!("invalid OCI specification: {}", err),
		)
	}
}

impl<E> From<SdkError<E>> for RegistryError
where
	E: StdError + Send + Sync + 'static,
{
	fn from(err: SdkError<E>) -> Self {
		error!("S3 error: {:?}", err.into_source());
		// Use Unsupported as a generic internal error code
		Self::builder()
			.code(ErrorCode::Unsupported)
			.message(
				if cfg!(debug_assertions) {
					"internal server error: S3 operation failed"
				} else {
					"internal server error"
				},
			)
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.build()
	}
}

impl From<ByteStreamError> for RegistryError {
	fn from(err: ByteStreamError) -> Self {
		error!("S3 ByteStream error: {:?}", err);
		// Use Unsupported as a generic internal error code
		Self::builder()
			.code(ErrorCode::Unsupported)
			.message(
				if cfg!(debug_assertions) {
					"internal server error: S3 ByteStream operation failed"
				} else {
					"internal server error"
				},
			)
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.build()
	}
}

impl From<PresigningConfigError> for RegistryError {
	fn from(err: PresigningConfigError) -> Self {
		error!("S3 PresigningConfig error: {:?}", err);
		// Use Unsupported as a generic internal error code
		Self::builder()
			.code(ErrorCode::Unsupported)
			.message(
				if cfg!(debug_assertions) {
					"internal server error: S3 PresigningConfig operation failed"
				} else {
					"internal server error"
				},
			)
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.build()
	}
}

impl From<headers::Error> for RegistryError {
	fn from(err: headers::Error) -> Self {
		error!("Header error: {}", err);
		Self::new(
			ErrorCode::Unsupported,
			format!("invalid HTTP header: {}", err),
		)
	}
}

impl From<rustis::Error> for RegistryError {
	fn from(err: rustis::Error) -> Self {
		error!("Redis error: {}", err);
		// Use Unsupported as a generic internal error code
		Self::builder()
			.code(ErrorCode::Unsupported)
			.message(
				if cfg!(debug_assertions) {
					format!("internal server error: Redis operation failed: {}", err)
				} else {
					"internal server error".to_string()
				},
			)
			.status(StatusCode::INTERNAL_SERVER_ERROR)
			.build()
	}
}

// Note: We don't implement a generic From<E> for all error types because it
// would conflict with the blanket implementation From<T> for T in core.
// Instead, we provide specific From implementations for common error types
// above, and a helper method for converting arbitrary errors.

// Implement IntoResponse for axum integration
impl IntoResponse for RegistryError {
	fn into_response(self) -> Response {
		let mut errors = ErrorInfoBuilder::default()
			.code(self.code.clone())
			.message(self.message.clone());

		if let Some(detail) = self.detail.clone() {
			errors = errors.detail(detail);
		}

		let oci_response = ErrorResponseBuilder::default()
			.errors(vec![errors.build().expect("Failed to build ErrorInfo")])
			.build()
			.expect("Failed to build ErrorResponse");
		let status = self.status;

		warn!(
			status = %status,
			code = ?self.code,
			message = %self.message,
			self = ?self,
			"Registry error response"
		);

		(status, Json(oci_response)).into_response()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_error_code_to_status_mapping() {
		// Test that error codes map to correct HTTP status codes
		assert_eq!(
			RegistryError::error_code_to_status(&ErrorCode::BlobUnknown),
			StatusCode::NOT_FOUND
		);
		assert_eq!(
			RegistryError::error_code_to_status(&ErrorCode::ManifestUnknown),
			StatusCode::NOT_FOUND
		);
		assert_eq!(
			RegistryError::error_code_to_status(&ErrorCode::Unauthorized),
			StatusCode::UNAUTHORIZED
		);
		assert_eq!(
			RegistryError::error_code_to_status(&ErrorCode::Denied),
			StatusCode::FORBIDDEN
		);
		assert_eq!(
			RegistryError::error_code_to_status(&ErrorCode::DigestInvalid),
			StatusCode::BAD_REQUEST
		);
	}

	#[test]
	fn test_new_error() {
		let err = RegistryError::new(ErrorCode::BlobUnknown, "test error");
		assert_eq!(err.status, StatusCode::NOT_FOUND);
		assert_eq!(err.message, "test error");
	}

	#[test]
	fn test_to_oci_error_response() {
		let err = RegistryError::new(ErrorCode::BlobUnknown, "blob not found");
		let oci_response = ErrorResponseBuilder::default()
			.errors(vec![
				ErrorInfoBuilder::default()
					.code(err.code.clone())
					.message(err.message.clone())
					.detail(err.detail.clone().unwrap_or_default())
					.build()
					.expect("Failed to build ErrorInfo"),
			])
			.build()
			.expect("Failed to build ErrorResponse");

		assert_eq!(oci_response.errors().len(), 1);
		let error_info = &oci_response.errors()[0];
		assert_eq!(error_info.code(), &ErrorCode::BlobUnknown);
		assert_eq!(error_info.message(), &Some("blob not found".to_string()));
	}

	#[test]
	fn test_from_serde_json_error() {
		let json_err =
			serde_json::from_str::<serde_json::Value>("invalid json").expect_err("should fail");
		let reg_err = RegistryError::from(json_err);
		assert_eq!(reg_err.status, StatusCode::BAD_REQUEST);
	}

	#[test]
	fn test_display() {
		let err = RegistryError::new(ErrorCode::BlobUnknown, "test error");
		let display = format!("{}", err);
		assert!(display.contains("BlobUnknown"));
		assert!(display.contains("test error"));
	}
}
