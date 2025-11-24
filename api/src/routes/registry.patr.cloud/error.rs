//! Registry error types.
//!
//! This module provides error handling for the OCI registry,
//! wrapping oci-spec error codes and providing conversions to HTTP responses.

use std::fmt;

use axum::{
	Json,
	http::StatusCode,
	response::{IntoResponse, Response},
};
use oci_spec::distribution::{ErrorCode, ErrorInfoBuilder, ErrorResponse, ErrorResponseBuilder};
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
	#[builder(default)]
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

	/// Create a new registry error with additional detail.
	///
	/// # Arguments
	///
	/// * `code` - The OCI error code
	/// * `message` - Human-readable error message
	/// * `detail` - Additional detailed information
	pub fn with_detail(
		code: ErrorCode,
		message: impl Into<String>,
		detail: impl Into<String>,
	) -> Self {
		let status = Self::error_code_to_status(&code);
		Self {
			code,
			message: message.into(),
			detail: Some(detail.into()),
			status,
		}
	}

	/// Create a new registry error with a custom status code.
	///
	/// # Arguments
	///
	/// * `code` - The OCI error code
	/// * `message` - Human-readable error message
	/// * `status` - HTTP status code to return
	pub fn with_status(code: ErrorCode, message: impl Into<String>, status: StatusCode) -> Self {
		Self {
			code,
			message: message.into(),
			detail: None,
			status,
		}
	}

	/// Convert the error to an OCI ErrorResponse.
	pub fn to_oci_error_response(&self) -> ErrorResponse {
		ErrorResponseBuilder::default()
			.errors(vec![
				ErrorInfoBuilder::default()
					.code(self.code.clone())
					.message(self.message.clone())
					.detail(self.detail.clone().unwrap_or_default())
					.build()
					.expect("Failed to build ErrorInfo"),
			])
			.build()
			.expect("Failed to build ErrorResponse")
	}

	/// Get the HTTP status code for this error.
	pub fn status_code(&self) -> StatusCode {
		self.status
	}

	/// Map OCI error codes to HTTP status codes.
	fn error_code_to_status(code: &ErrorCode) -> StatusCode {
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

	// Convenience constructors for common errors

	/// Create a BLOB_UNKNOWN error.
	pub fn blob_unknown(digest: impl Into<String>) -> Self {
		Self::new(
			ErrorCode::BlobUnknown,
			format!("blob {} not found", digest.into()),
		)
	}

	/// Create a MANIFEST_UNKNOWN error.
	pub fn manifest_unknown(reference: impl Into<String>) -> Self {
		Self::new(
			ErrorCode::ManifestUnknown,
			format!("manifest {} not found", reference.into()),
		)
	}

	/// Create a NAME_INVALID error.
	pub fn name_invalid(name: impl Into<String>) -> Self {
		Self::new(
			ErrorCode::NameInvalid,
			format!("invalid repository name: {}", name.into()),
		)
	}

	/// Create a NAME_UNKNOWN error.
	pub fn name_unknown(name: impl Into<String>) -> Self {
		Self::new(
			ErrorCode::NameUnknown,
			format!("repository {} not found", name.into()),
		)
	}

	/// Create a DIGEST_INVALID error.
	pub fn digest_invalid(digest: impl Into<String>) -> Self {
		Self::new(
			ErrorCode::DigestInvalid,
			format!("invalid digest: {}", digest.into()),
		)
	}

	/// Create an UNAUTHORIZED error.
	pub fn unauthorized(message: impl Into<String>) -> Self {
		Self::new(ErrorCode::Unauthorized, message)
	}

	/// Create a DENIED error.
	pub fn denied(message: impl Into<String>) -> Self {
		Self::new(ErrorCode::Denied, message)
	}

	/// Create a BLOB_UPLOAD_INVALID error.
	pub fn blob_upload_invalid(message: impl Into<String>) -> Self {
		Self::new(ErrorCode::BlobUploadInvalid, message)
	}

	/// Create a BLOB_UPLOAD_UNKNOWN error.
	pub fn blob_upload_unknown(uuid: impl Into<String>) -> Self {
		Self::new(
			ErrorCode::BlobUploadUnknown,
			format!("upload session {} not found", uuid.into()),
		)
	}

	/// Create a MANIFEST_INVALID error.
	pub fn manifest_invalid(message: impl Into<String>) -> Self {
		Self::new(ErrorCode::ManifestInvalid, message)
	}

	/// Create a MANIFEST_BLOB_UNKNOWN error.
	pub fn manifest_blob_unknown(digest: impl Into<String>) -> Self {
		Self::new(
			ErrorCode::ManifestBlobUnknown,
			format!("manifest references unknown blob: {}", digest.into()),
		)
	}

	/// Create an UNSUPPORTED error.
	pub fn unsupported(message: impl Into<String>) -> Self {
		Self::new(ErrorCode::Unsupported, message)
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

impl std::error::Error for RegistryError {}

// Implement From<E> where E: std::error::Error for common error types
impl From<sqlx::Error> for RegistryError {
	fn from(err: sqlx::Error) -> Self {
		error!("Database error: {}", err);
		// Use Unsupported as a generic internal error code
		Self::with_status(
			ErrorCode::Unsupported,
			"internal server error: database operation failed",
			StatusCode::INTERNAL_SERVER_ERROR,
		)
	}
}

impl From<std::io::Error> for RegistryError {
	fn from(err: std::io::Error) -> Self {
		error!("I/O error: {}", err);
		// Use Unsupported as a generic internal error code
		Self::with_status(
			ErrorCode::Unsupported,
			"internal server error: I/O operation failed",
			StatusCode::INTERNAL_SERVER_ERROR,
		)
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

impl From<aws_sdk_s3::Error> for RegistryError {
	fn from(err: aws_sdk_s3::Error) -> Self {
		error!("S3 error: {}", err);
		// Use Unsupported as a generic internal error code
		Self::with_status(
			ErrorCode::Unsupported,
			"internal server error: S3 operation failed",
			StatusCode::INTERNAL_SERVER_ERROR,
		)
	}
}

// Note: We don't implement a generic From<E> for all error types because it
// would conflict with the blanket implementation From<T> for T in core.
// Instead, we provide specific From implementations for common error types
// above, and a helper method for converting arbitrary errors.

// Implement IntoResponse for axum integration
impl IntoResponse for RegistryError {
	fn into_response(self) -> Response {
		let status = self.status_code();
		let oci_response = self.to_oci_error_response();

		warn!(
			status = %status,
			code = ?self.code,
			message = %self.message,
			"Registry error response"
		);

		let mut response = (status, Json(oci_response)).into_response();

		// Add WWW-Authenticate header for 401 Unauthorized responses
		// as required by the OCI Distribution Specification
		if status == StatusCode::UNAUTHORIZED {
			response.headers_mut().insert(
				axum::http::header::WWW_AUTHENTICATE,
				axum::http::HeaderValue::from_static("Bearer realm=\"registry.patr.cloud\""),
			);
		}

		response
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
		assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
		assert_eq!(err.message, "test error");
	}

	#[test]
	fn test_error_with_detail() {
		let err = RegistryError::with_detail(
			ErrorCode::ManifestInvalid,
			"invalid manifest",
			"missing required field",
		);
		assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
		assert_eq!(err.message, "invalid manifest");
		assert_eq!(err.detail, Some("missing required field".to_string()));
	}

	#[test]
	fn test_error_with_custom_status() {
		let err = RegistryError::with_status(
			ErrorCode::Unsupported,
			"custom error",
			StatusCode::SERVICE_UNAVAILABLE,
		);
		assert_eq!(err.status_code(), StatusCode::SERVICE_UNAVAILABLE);
	}

	#[test]
	fn test_convenience_constructors() {
		let err = RegistryError::blob_unknown("sha256:abc123");
		assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
		assert!(err.message.contains("sha256:abc123"));

		let err = RegistryError::manifest_unknown("latest");
		assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
		assert!(err.message.contains("latest"));

		let err = RegistryError::name_invalid("invalid/name");
		assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
		assert!(err.message.contains("invalid/name"));

		let err = RegistryError::unauthorized("invalid token");
		assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);

		let err = RegistryError::denied("insufficient permissions");
		assert_eq!(err.status_code(), StatusCode::FORBIDDEN);
	}

	#[test]
	fn test_to_oci_error_response() {
		let err = RegistryError::new(ErrorCode::BlobUnknown, "blob not found");
		let oci_response = err.to_oci_error_response();

		assert_eq!(oci_response.errors().len(), 1);
		let error_info = &oci_response.errors()[0];
		assert_eq!(error_info.code(), &ErrorCode::BlobUnknown);
		assert_eq!(error_info.message(), &Some("blob not found".to_string()));
	}

	#[test]
	fn test_to_oci_error_response_with_detail() {
		let err = RegistryError::with_detail(
			ErrorCode::ManifestInvalid,
			"invalid manifest",
			"missing schemaVersion",
		);
		let oci_response = err.to_oci_error_response();

		assert_eq!(oci_response.errors().len(), 1);
		let error_info = &oci_response.errors()[0];
		assert_eq!(error_info.code(), &ErrorCode::ManifestInvalid);
		assert_eq!(error_info.message(), &Some("invalid manifest".to_string()));
		assert_eq!(
			error_info.detail(),
			&Some("missing schemaVersion".to_string())
		);
	}

	#[test]
	fn test_from_serde_json_error() {
		let json_err =
			serde_json::from_str::<serde_json::Value>("invalid json").expect_err("should fail");
		let reg_err = RegistryError::from(json_err);
		assert_eq!(reg_err.status_code(), StatusCode::BAD_REQUEST);
	}

	#[test]
	fn test_from_io_error() {
		let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
		let reg_err = RegistryError::from(io_err);
		assert_eq!(reg_err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
	}

	#[test]
	fn test_display() {
		let err = RegistryError::new(ErrorCode::BlobUnknown, "test error");
		let display = format!("{}", err);
		assert!(display.contains("BlobUnknown"));
		assert!(display.contains("test error"));
	}
}
