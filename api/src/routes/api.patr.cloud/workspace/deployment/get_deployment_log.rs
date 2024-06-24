use std::{cmp::Ordering, collections::BTreeMap};

use axum::http::StatusCode;
use models::api::workspace::deployment::*;
use time::OffsetDateTime;

use crate::prelude::*;

/// Get deployment logs
///
/// #Parameters
/// - `workspace_id`: The workspace ID
/// - `deployment_id`: The deployment ID
/// - `end_time`: The end time
/// - `limit`: The interval of the logs required
///
/// #Returns
/// - `logs`: The logs
pub async fn get_deployment_log(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetDeploymentLogPath {
					workspace_id,
					deployment_id,
				},
				query: GetDeploymentLogQuery { end_time, limit },
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, GetDeploymentLogRequest>,
) -> Result<AppResponse<GetDeploymentLogRequest>, ErrorType> {
	info!("Getting logs for deployment: {}", deployment_id);

	// LOGIC

	AppResponse::builder()
		.body(GetDeploymentLogResponse { logs: todo!() })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
