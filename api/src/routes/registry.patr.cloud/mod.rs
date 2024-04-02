use std::fmt::Display;

use axum::{
	body::Body,
	http::{header::InvalidHeaderValue, StatusCode},
	response::{IntoResponse, Response},
	routing::get,
	Router,
};
use s3::{creds::error::CredentialsError, error::S3Error};
use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Download a specific blob, given its digest.
mod get_blob_info;
/// Get the status of the registry.
mod get_registry_status;

/// The error type for the registry routes. This is used to return errors in the
/// registry. The error details are taken from the Docker Registry API v2
/// specification at https://github.com/opencontainers/distribution-spec/blob/main/spec.md#error-codes
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RegistryError {
	/// Blob unknown to registry
	BlobUnknown,
	/// Blob upload invalid
	BlobUploadInvalid,
	/// Blob upload unknown to registry
	BlobUploadUnknown,
	/// Provided digest did not match uploaded content
	DigestInvalid,
	/// Manifest references a manifest or blob unknown to registry
	ManifestBlobUnknown,
	/// Manifest invalid
	ManifestInvalid,
	/// Manifest unknown to registry
	ManifestUnknown,
	/// Invalid repository name
	NameInvalid,
	/// Repository name not known to registry
	NameUnknown,
	/// Provided length did not match content length
	SizeInvalid,
	/// Authentication required
	Unauthorized,
	/// Requested access to the resource is denied
	Denied,
	/// The operation is unsupported
	Unsupported,
	/// Too many requests
	#[serde(rename = "TOOMANYREQUESTS")]
	TooManyRequests,
	/// Internal server error
	InternalServerError,
}

/// The error response for the registry routes. This is used to return errors in
/// the registry. The error details are taken from the Docker Registry API v2
/// specification at https://github.com/opencontainers/distribution-spec/blob/main/spec.md#error-codes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Error {
	/// The list of errors that occurred. According to the spec, this would be a
	/// list of errors that occurred during the request. However, we only return
	/// one error at a time.
	pub errors: [ErrorItem; 1],
}

impl IntoResponse for Error {
	fn into_response(self) -> Response {
		let Error {
			errors: [ErrorItem {
				code,
				message: _,
				detail: _,
			}],
		} = self;
		Response::builder()
			.status(match code {
				RegistryError::BlobUnknown => StatusCode::NOT_FOUND,
				RegistryError::BlobUploadInvalid => todo!(),
				RegistryError::BlobUploadUnknown => todo!(),
				RegistryError::DigestInvalid => todo!(),
				RegistryError::ManifestBlobUnknown => todo!(),
				RegistryError::ManifestInvalid => todo!(),
				RegistryError::ManifestUnknown => todo!(),
				RegistryError::NameInvalid => todo!(),
				RegistryError::NameUnknown => todo!(),
				RegistryError::SizeInvalid => todo!(),
				RegistryError::Unauthorized => todo!(),
				RegistryError::Denied => todo!(),
				RegistryError::Unsupported => todo!(),
				RegistryError::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
				RegistryError::InternalServerError => {
					return StatusCode::INTERNAL_SERVER_ERROR.into_response()
				}
			})
			.body(Body::empty())
			.unwrap_or_else(|_| {
				// If we can't create the response, just return an empty response
				(StatusCode::INTERNAL_SERVER_ERROR, Body::empty()).into_response()
			})
	}
}

impl From<S3Error> for Error {
	fn from(err: S3Error) -> Self {
		error(RegistryError::InternalServerError, err)
	}
}

impl From<sqlx::Error> for Error {
	fn from(err: sqlx::Error) -> Self {
		error(RegistryError::InternalServerError, err)
	}
}

impl From<CredentialsError> for Error {
	fn from(err: CredentialsError) -> Self {
		error(RegistryError::InternalServerError, err)
	}
}

impl From<InvalidHeaderValue> for Error {
	fn from(err: InvalidHeaderValue) -> Self {
		error(RegistryError::InternalServerError, err)
	}
}

/// The error item for the registry routes. This contains the specific error in
/// the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorItem {
	/// The error code that occurred.
	pub code: RegistryError,
	/// The message for the error.
	pub message: String,
	/// The detail for the error, if any. If none, this will be an empty string.
	#[serde(default)]
	pub detail: String,
}

/// Create an error response with the given error and message.
fn error(error: RegistryError, message: impl Display) -> Error {
	Error {
		errors: [ErrorItem {
			code: error,
			message: message.to_string(),
			detail: "".to_string(),
		}],
	}
}

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.nest(
			"/v2",
			Router::new()
				.route("/", get(get_registry_status::handle))
				.route(
					"/:workspaceId/:repoName/blobs/:digest",
					get(get_blob_info::handle).head(get_blob_info::handle),
				), /* .route(
			    * 	"/:workspaceId/:repoName/blobs/uploads/",
			    * 	post(get_push_session_id::handle),
			    * )
			    * .route(
			    * 	"/:workspaceId/:repoName/blobs/:digest",
			    * 	head(get_blob_info::handle).get(get_blob::handle),
			    * )
			    * .route(
			    * 	"/:workspaceId/:repoName/manifests/:tag",
			    * 	put(add_manifest_to_repo),
			    * )
			    * .route(
			    * 	"/:workspaceId/:repoName/blobs/uploads/:digest",
			    * 	patch(patch_blob),
			    * ), */
		)
		.with_state(state.clone())
}
