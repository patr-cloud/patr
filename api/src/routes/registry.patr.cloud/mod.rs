use std::fmt::Display;

use axum::{routing::get, Router};
use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// Download a specific blob, given its digest.
mod get_blob_info;
/// Get the status of the registry.
mod get_registry_status;

/// The error type for the registry routes. This is used to return errors in the
/// registry. The error details are taken from the Docker Registry API v2
/// specification at https://github.com/opencontainers/distribution-spec/blob/main/spec.md#error-codes
#[derive(Debug, Clone, Serialize, Deserialize)]
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
