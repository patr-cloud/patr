use axum::http::StatusCode;
use models::{api::workspace::infrastructure::managed_url::*, prelude::*};

use crate::prelude::*;

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
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, VerifyManagedURLConfigurationRequest>,
) -> Result<AppResponse<VerifyManagedURLConfigurationRequest>, ErrorType> {
	info!("Verifying configuration of ManagedURL");

	AppResponse::builder()
		.body(VerifyManagedURLConfigurationResponse {
			configured: panic!("Check if the managed URL is configured correctly"),
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
