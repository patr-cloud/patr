//! DELETE blob endpoint handler.
//!
//! This handler processes DELETE requests to remove a blob from the registry.
//! The blob is deleted through the Patr API directly, as blob deletion
//! is currently disabled in the registry endpoints.

use crate::routes::registry_patr_cloud::prelude::*;

macros::declare_registry_endpoint!(
	/// DELETE blob endpoint.
	///
	/// Deletes a blob from the specified repository. Currently disabled;
	/// blob deletions are handled through the Patr API directly. This
	/// endpoint will return 405 Method Not Allowed.
	DeleteBlob,
	DELETE "/v2/{repo_name}/blobs/{reference}" {
		/// The repository name
		#[preprocess(length(max = 255))]
		pub repo_name: String,
		/// The blob reference (tag name or digest)
		#[preprocess(regex = constants::REGISTRY_TAG_OR_DIGEST_REGEX)]
		pub reference: String,
	},
	request_headers = {
		/// Authorization header with Bearer token
		pub authorization: BearerToken,
	},
);

/// Handler for DELETE /v2/{repo_name}/blobs/{reference}
///
/// This handler will return a constant 405 Method Not Allowed response,
/// as blob deletion is currently disabled. Blob deletions are done
/// through the Patr API directly.
pub async fn delete_blob(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path: DeleteBlobPathProcessed {
					repo_name,
					reference,
				},
				query: (),
				headers: DeleteBlobRequestHeaders { authorization: _ },
				body: _,
			},
		database: _,
		redis: _,
		s3: _,
		client_ip: _,
		user_data,
		config: _,
	}: AuthenticatedRegistryAppRequest<'_, DeleteBlobPath>,
) -> Result<RegistryResponse<DeleteBlobPath>, RegistryError> {
	debug!(
		repo_name = %repo_name,
		reference = %reference,
		user_id = %user_data.id,
		"Manifest deletion requested is currently disabled"
	);

	RegistryResponse::builder()
		.body(Body::empty())
		.status_code(StatusCode::METHOD_NOT_ALLOWED)
		.headers(())
		.build()
		.into_result()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_delete_blob_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<DeleteBlobPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{name}/blobs/{digest}"
		);
	}
}
