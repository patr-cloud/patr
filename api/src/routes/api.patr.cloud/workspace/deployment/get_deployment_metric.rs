use axum::http::StatusCode;
use models::api::workspace::deployment::*;

use crate::prelude::*;

/// Route to get the metrics of a deployment. This will fetch metrics from Mimir
/// and return them to the user. The metrics can be filtered by the end time.
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
						authorization: _,
						user_agent: _,
					},
				body: GetDeploymentMetricRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data: _,
	}: AuthenticatedAppRequest<'_, GetDeploymentMetricRequest>,
) -> Result<AppResponse<GetDeploymentMetricRequest>, ErrorType> {
	info!(
		"Getting deployment metrics for deployment: {}",
		deployment_id
	);

	query!(
		r#"
		SELECT
			id
		FROM
			deployment
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		deployment_id as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	AppResponse::builder()
		.body(GetDeploymentMetricResponse { metrics: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
