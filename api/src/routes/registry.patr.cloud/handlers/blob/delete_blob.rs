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
	DELETE "/v2/{workspace_id}/{repo_name}/blobs/{digest}" {
		/// The workspace ID
		pub workspace_id: Uuid,
		/// The repository name
		#[preprocess(lowercase, regex = constants::REGISTRY_REPO_NAME_REGEX, length(max = 255))]
		pub repo_name: String,
		/// The blob digest
		#[preprocess(regex = constants::REGISTRY_DIGEST_REGEX)]
		pub digest: String,
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
				path:
					DeleteBlobPathProcessed {
						workspace_id: _,
						repo_name: _,
						digest: _,
					},
				query: (),
				headers: DeleteBlobRequestHeaders { authorization: _ },
				body: _,
			},
		database: _,
		redis: _,
		s3: _,
		client_ip: _,
		user_data: _,
		config: _,
	}: AuthenticatedRegistryAppRequest<'_, DeleteBlobPath>,
) -> Result<RegistryResponse<DeleteBlobPath>, RegistryError> {
	debug!("Manifest deletion requested is currently disabled");

	RegistryResponse::builder()
		.body(Body::empty())
		.status_code(StatusCode::METHOD_NOT_ALLOWED)
		.headers(())
		.build()
		.into_result()
}
