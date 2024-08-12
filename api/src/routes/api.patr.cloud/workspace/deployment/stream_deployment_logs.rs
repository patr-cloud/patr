use axum::http::StatusCode;
use axum_typed_websockets::Message;
use models::{
	api::workspace::deployment::*,
	utils::{GenericResponse, WebSocketUpgrade},
};
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
pub async fn stream_deployment_log(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: StreamDeploymentLogsPath {
					workspace_id,
					deployment_id,
				},
				query: StreamDeploymentLogsQuery { end_time, limit },
				headers,
				body: WebSocketUpgrade(upgrade),
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, StreamDeploymentLogsRequest>,
) -> Result<AppResponse<StreamDeploymentLogsRequest>, ErrorType> {
	info!("Streaming logs for deployment: {}", deployment_id);

	AppResponse::builder()
		.body(GenericResponse(
			upgrade
				.on_upgrade(move |mut websocket| async move {
					loop {
						let Ok(()) = websocket
							.send(Message::Item(StreamDeploymentLogsServerMsg::LogData {
								log: DeploymentLogs {
									timestamp: OffsetDateTime::now_utc(),
									logs: format!("Log data for {}", OffsetDateTime::now_utc()),
								},
							}))
							.await
						else {
							debug!("Failed to send data to websocket");
							break;
						};
					}
				})
				.into_response(),
		))
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
