use axum::{http::StatusCode, response::IntoResponse};
use axum_typed_websockets::Message;
use futures::StreamExt;
use headers::{Authorization, HeaderMapExt};
use models::{
	api::workspace::deployment::*,
	utils::{GenericResponse, WebSocketUpgrade},
};
use reqwest::Method;
use time::OffsetDateTime;
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message as RawMessage};

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LokiResponse {
	streams: LokiStreams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LokiStreams {
	values: Vec<(i128, String)>,
}

/// Route to stream the logs of a deployment. This will stream logs from Loki
/// and return them to the user. The logs can be filtered by the start time.
pub async fn stream_deployment_logs(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: StreamDeploymentLogsPath {
					workspace_id,
					deployment_id,
				},
				query: StreamDeploymentLogsQuery { start_time },
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

	#[cfg(debug_assertions)]
	let Some(loki) = config.loki
	else {
		return Err(ErrorType::server_error("Loki configuration not found"));
	};
	#[cfg(not(debug_assertions))]
	let loki = config.loki;

	let mut client_request = format!(
		"{}://{}",
		if loki.endpoint.starts_with("https") {
			"wss"
		} else {
			"ws"
		},
		loki.endpoint
			.trim_start_matches("https://")
			.trim_start_matches("http://"),
	)
	.into_client_request()
	.map_err(|err| ErrorType::server_error(err.to_string()))?;
	client_request
		.headers_mut()
		.typed_insert(Authorization::basic(&loki.username, &loki.password));
	*client_request.method_mut() = Method::GET;
	*client_request.uri_mut().query() = Some(serde_urlencoded::to_string(&[(
		"start",
		start_time
			.unwrap_or(OffsetDateTime::now_utc())
			.unix_timestamp_nanos()
			.to_string(),
	)]))?;

	let (mut stream, _) = tokio_tungstenite::connect_async(client_request)
		.await
		.inspect_err(|err| error!("Failed to stream from Loki: {}", err))?;

	AppResponse::builder()
		.body(GenericResponse(
			upgrade
				.on_upgrade(move |mut websocket| async move {
					while let Some(data) = stream.next().await {
						let Ok(data) = data.inspect_err(|err| {
							debug!("Failed to get data from Loki: {}", err);
						}) else {
							break;
						};

						let bytes = match data {
							RawMessage::Text(text) => text.into_bytes(),
							RawMessage::Binary(bin) => bin,
							RawMessage::Close(_) => break,
							_ => continue,
						};

						let Ok(message) = serde_json::from_slice::<LokiResponse>(&bytes)
							.inspect_err(|err| {
								debug!("Failed to parse Loki message: {}", err);
							})
						else {
							break;
						};

						let logs = message
							.streams
							.values
							.into_iter()
							.map(|(timestamp, log)| DeploymentLogs {
								timestamp: OffsetDateTime::from_unix_timestamp_nanos(timestamp),
								log,
							})
							.collect();

						let Ok(()) = websocket
							.send(Message::Item(StreamDeploymentLogsServerMsg::LogData {
								logs,
							}))
							.await
							.inspect_err(|err| {
								debug!("Failed to send logs to client: {}", err);
							})
						else {
							break;
						};
					}
					_ = websocket.send(Message::Close(None)).await;
					_ = websocket.close().await;
				})
				.into_response(),
		))
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
