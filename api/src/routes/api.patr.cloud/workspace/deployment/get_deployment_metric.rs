use axum::http::StatusCode;
use models::api::workspace::deployment::*;

use crate::prelude::*;

/// Get deployment metrics
///
/// #Parameters
/// - `workspace_id`: The workspace ID
/// - `deployment_id`: The deployment ID
///
/// #Returns
/// - `mertrics`: The deployment metrics
pub async fn get_deployment_metric(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetDeploymentMetricPath {
					workspace_id,
					deployment_id,
				},
				query: GetDeploymentMetricQuery { end_time, limit },
				headers:
					GetDeploymentMetricRequestHeaders {
						authorization,
						user_agent,
					},
				body: GetDeploymentMetricRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, GetDeploymentMetricRequest>,
) -> Result<AppResponse<GetDeploymentMetricRequest>, ErrorType> {
	info!("Starting: Get deployment metrics");

	// LOGIC

	AppResponse::builder()
		.body(GetDeploymentMetricResponse { metrics: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
