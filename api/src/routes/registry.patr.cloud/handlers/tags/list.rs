//! Tags-list endpoint handler (stub).
//!
//! Listing tags over the OCI API is intentionally unimplemented — tag listing
//! for Patr is done through the Patr API. This endpoint returns
//! 405 Method Not Allowed, which OCI clients (and the conformance suite) treat
//! as "feature unsupported".

use crate::routes::registry_patr_cloud::prelude::*;

macros::declare_registry_endpoint!(
	/// GET tags-list endpoint.
	///
	/// Intentionally unimplemented (listing is via the Patr API). Returns a
	/// constant 405 Method Not Allowed.
	ListTags,
	GET "/v2/{workspace_id}/{repo_name}/tags/list" {
		/// The workspace ID
		pub workspace_id: Uuid,
		/// The repository name
		#[preprocess(lowercase, regex = constants::REGISTRY_REPO_NAME_REGEX, length(max = 255))]
		pub repo_name: String,
	},
	request_headers = {
		/// Authorization header with Bearer token
		pub authorization: BearerToken,
	},
);

/// Handler for GET /v2/{workspace_id}/{repo_name}/tags/list
///
/// Returns a constant 405 Method Not Allowed — tag listing is handled through
/// the Patr API, not the OCI registry endpoints.
pub async fn list_tags(
	AuthenticatedRegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path: ListTagsPathProcessed {
					workspace_id: _,
					repo_name: _,
				},
				query: (),
				headers: ListTagsRequestHeaders { authorization: _ },
				body: _,
			},
		database: _,
		redis: _,
		s3: _,
		client_ip: _,
		user_data: _,
		config: _,
	}: AuthenticatedRegistryAppRequest<'_, ListTagsPath>,
) -> Result<RegistryResponse<ListTagsPath>, RegistryError> {
	debug!("Tag listing over the OCI API is intentionally disabled");

	RegistryResponse::builder()
		.body(Body::empty())
		.status_code(StatusCode::METHOD_NOT_ALLOWED)
		.headers(())
		.build()
		.into_result()
}
