use axum::http::StatusCode;
use models::api::*;

use crate::prelude::*;

pub async fn get_api_environment(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: GetApiEnvironmentPath,
				query: (),
				headers: GetApiEnvironmentRequestHeaders { user_agent: _ },
				body: GetApiEnvironmentRequestProcessed,
			},
		database: _,
		redis: _,
		client_ip: _,
		state,
	}: AppRequest<'_, GetApiEnvironmentRequest>,
) -> Result<AppResponse<GetApiEnvironmentRequest>, ErrorType> {
	let deployment_type = if cfg!(feature = "cloud") {
		DeploymentType::Cloud
	} else {
		DeploymentType::SelfHosted
	};

	// Only self-hosted clients can't know the base domain at build time —
	// cloud bundles hard-code `patr.cloud`. Emit nothing on cloud so the
	// payload stays unambiguous.
	let base_domain = if cfg!(feature = "cloud") {
		None
	} else {
		Some(state.config.server.base_domain.clone())
	};

	AppResponse::builder()
		.body(GetApiEnvironmentResponse {
			version: env!("CARGO_PKG_VERSION").to_string(),
			deployment_type,
			base_domain,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
