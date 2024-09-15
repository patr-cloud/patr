use axum::http::StatusCode;
use models::api::workspace::deployment::*;
use time::{Duration, OffsetDateTime};

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
pub async fn get_deployment_logs(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetDeploymentLogsPath {
					workspace_id,
					deployment_id,
				},
				query: GetDeploymentLogsQuery { end_time, limit },
				headers:
					GetDeploymentLogsRequestHeaders {
						authorization,
						user_agent,
					},
				body: GetDeploymentLogsRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, GetDeploymentLogsRequest>,
) -> Result<AppResponse<GetDeploymentLogsRequest>, ErrorType> {
	info!("Getting logs for deployment: {}", deployment_id);

	// LOGIC
	let logs = vec![
		DeploymentLogs {
			timestamp: OffsetDateTime::now_utc() - Duration::seconds(10),
			logs: "Some random log data 10".to_string(),
		},
		DeploymentLogs {
			timestamp: OffsetDateTime::now_utc() - Duration::seconds(9),
			logs: "Some random log data 9".to_string(),
		},
		DeploymentLogs {
			timestamp: OffsetDateTime::now_utc() - Duration::seconds(8),
			logs: "Some random log data 8".to_string(),
		},
		DeploymentLogs {
			timestamp: OffsetDateTime::now_utc() - Duration::seconds(7),
			logs: "Some random log data 7".to_string(),
		},
		DeploymentLogs {
			timestamp: OffsetDateTime::now_utc() - Duration::seconds(6),
			logs: "Some random log data 6".to_string(),
		},
		DeploymentLogs {
			timestamp: OffsetDateTime::now_utc() - Duration::seconds(5),
			logs: "Some random log data 5".to_string(),
		},
		DeploymentLogs {
			timestamp: OffsetDateTime::now_utc() - Duration::seconds(4),
			logs: "Some random log data 4".to_string(),
		},
		DeploymentLogs {
			timestamp: OffsetDateTime::now_utc() - Duration::seconds(3),
			logs: "Some random log data 3".to_string(),
		},
		DeploymentLogs {
			timestamp: OffsetDateTime::now_utc() - Duration::seconds(2),
			logs: "Some random log data 2".to_string(),
		},
		DeploymentLogs {
			timestamp: OffsetDateTime::now_utc() - Duration::seconds(1),
			logs: "Some random log data 1".to_string(),
		},
		DeploymentLogs {
			timestamp: OffsetDateTime::now_utc() - Duration::seconds(0),
			logs: "Some random log data 0".to_string(),
		},
	];

	AppResponse::builder()
		.body(GetDeploymentLogsResponse { logs })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
