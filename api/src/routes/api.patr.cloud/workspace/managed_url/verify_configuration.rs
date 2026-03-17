use axum::http::StatusCode;
use models::{api::workspace::managed_url::*, prelude::*};

use crate::prelude::*;

/// Verify the configuration of a managed URL
///
/// #Parameters
/// - `workspace_id`: The workspace ID
/// - `managed_url_id`: The managed URL ID
///
/// #Returns
/// - `configured`: TODO
pub async fn verify_configuration(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					VerifyManagedURLConfigurationPath {
						workspace_id: _,
						managed_url_id: _,
					},
				query: (),
				headers:
					VerifyManagedURLConfigurationRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: VerifyManagedURLConfigurationRequestProcessed,
			},
		database: _,
		redis: _,
		client_ip: _,
		user_data: _,
		state: _,
	}: AuthenticatedAppRequest<'_, VerifyManagedURLConfigurationRequest>,
) -> Result<AppResponse<VerifyManagedURLConfigurationRequest>, ErrorType> {
	info!("Verifying configuration of ManagedURL");

	// TODO: Check if the managed URL is configured correctly
	AppResponse::builder()
		.body(VerifyManagedURLConfigurationResponse { configured: false })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
