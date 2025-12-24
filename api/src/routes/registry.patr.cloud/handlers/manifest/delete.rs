//! DELETE manifest endpoint handler.
//!
//! This handler processes DELETE requests to remove a manifest from a
//! repository. The manifest is deleted through the Patr API directly, as
//! manifest deletion is currently disabled in the registry endpoints.

use crate::routes::registry_patr_cloud::prelude::*;

macros::declare_registry_endpoint!(
	/// DELETE manifest endpoint.
	///
	/// Deletes a manifest from the specified repository. Currently disabled;
	/// manifest deletions are handled through the Patr API directly. This
	/// endpoint will return 405 Method Not Allowed.
	DeleteManifest,
	DELETE "/v2/{repo_name}/manifests/{reference}" {
		/// The repository name
		#[preprocess(length(max = 255))]
		pub repo_name: String,
		/// The manifest reference (tag name or digest)
		#[preprocess(regex = constants::REGISTRY_TAG_OR_DIGEST_REGEX)]
		pub reference: String,
	},
	request_headers = {
		/// Authorization header with Bearer token
		pub authorization: BearerToken,
	},
);

/// Handler for DELETE /v2/{name}/manifests/{reference}
///
/// This handler will return a constant 405 Method Not Allowed response,
/// as manifest deletion is currently disabled. Manifest deletions are done
/// through the Patr API directly.
pub async fn delete_manifest(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path: DeleteManifestPathProcessed {
					repo_name,
					reference,
				},
				query: (),
				headers: DeleteManifestRequestHeaders { authorization: _ },
				body: _,
			},
		database: _,
		redis: _,
		s3: _,
		client_ip: _,
		user_data,
		config: _,
	}: AuthenticatedRegistryAppRequest<'_, DeleteManifestPath>,
) -> Result<RegistryResponse<DeleteManifestPath>, RegistryError> {
	debug!(
		repo_name = %repo_name,
		reference = %reference,
		user_id = %user_data.id,
		"Manifest deletion requested but currently disabled"
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
	fn test_delete_manifest_endpoint_path() {
		// Verify the endpoint path is correct
		assert_eq!(
			<DeleteManifestPath as axum_extra::routing::TypedPath>::PATH,
			"/v2/{name}/manifests/{reference}"
		);
	}
}
