use crate::routes::registry_patr_cloud::prelude::*;

macros::declare_registry_endpoint!(
	/// Version check endpoint (GET /v2/).
	///
	/// This endpoint provides OCI Distribution API version information.
	/// It does not require authentication and is used by clients to verify
	/// that the registry is accessible and supports the OCI Distribution API.
	GetApiVersion,
	GET "/v2/",
	auth = false,
	response_headers = {
		/// The Docker Distribution API version header
		pub version: DockerDistributionApiVersion,
	}
);

/// Handler for the version check endpoint.
///
/// Returns 200 OK with the Docker-Distribution-API-Version header set to
/// "registry/2.0".
#[instrument]
pub async fn version_check(
	RegistryAppRequest {
		request:
			RegistryProcessedApiRequest {
				path: GetApiVersionPathProcessed,
				query: (),
				headers: (),
				body: _,
			},
		database: _,
		redis: _,
		s3: _,
		client_ip: _,
		config: _,
	}: RegistryAppRequest<'_, GetApiVersionPath>,
) -> Result<RegistryResponse<GetApiVersionPath>, RegistryError> {
	debug!("Registry version check request");

	Ok(RegistryResponse::builder()
		.headers(GetApiVersionResponseHeaders {
			version: DockerDistributionApiVersion,
		})
		.status_code(StatusCode::OK)
		.body(Body::empty())
		.build())
}
