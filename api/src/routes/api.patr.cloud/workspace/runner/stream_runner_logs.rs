use std::{collections::BTreeMap, str::FromStr};

use axum::{
	http::{HeaderName, HeaderValue, StatusCode, Uri},
	response::IntoResponse,
};
use axum_typed_websockets::Message;
use futures::StreamExt;
use models::{
	api::workspace::runner::*,
	utils::{GenericResponse, WebSocketUpgrade},
};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio_tungstenite::tungstenite::{Message as RawMessage, client::IntoClientRequest};

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LokiResponse {
	streams: Vec<LokiStream>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LokiStream {
	values: Vec<(String, String)>,
}

/// Route to stream the logs of a runner process in real time. This connects
/// to Loki's tail endpoint filtered by runner_id and source="runner".
pub async fn stream_runner_logs(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: StreamRunnerLogsPath {
					workspace_id,
					runner_id,
				},
				query: StreamRunnerLogsQueryProcessed { start_time },
				headers:
					StreamRunnerLogsRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: WebSocketUpgrade(upgrade),
			},
		database: _,
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, StreamRunnerLogsRequest>,
) -> Result<AppResponse<StreamRunnerLogsRequest>, ErrorType> {
	info!("Streaming logs for runner: {}", runner_id);

	// Runner existence is already validated by ResourcePermissionAuthenticator

	let mut client_request = Uri::from_str(&format!(
		"{}://{}/loki/api/v1/tail?{}",
		if state
			.config
			.opentelemetry
			.logs
			.endpoint
			.starts_with("https")
		{
			"wss"
		} else {
			"ws"
		},
		state
			.config
			.opentelemetry
			.logs
			.endpoint
			.trim_start_matches("http://")
			.trim_start_matches("https://"),
		serde_qs::to_string(&BTreeMap::from([
			(
				"start",
				start_time
					.unwrap_or(OffsetDateTime::now_utc())
					.unix_timestamp_nanos()
					.to_string(),
			),
			(
				"query",
				format!("{{runner_id=\"{}\", source=\"runner\"}}", runner_id),
			),
		]))?
	))?
	.into_client_request()?;
	client_request.headers_mut().insert(
		HeaderName::from_static("x-scope-orgid"),
		HeaderValue::from_str(&workspace_id.to_string()).unwrap(),
	);
	*client_request.method_mut() = Method::GET;

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

						let Ok(message) = match data {
							RawMessage::Text(text) => serde_json::from_str::<LokiResponse>(&text),
							RawMessage::Binary(bin) => serde_json::from_slice::<LokiResponse>(&bin),
							RawMessage::Close(_) => break,
							_ => continue,
						}
						.inspect_err(|err| {
							debug!("Failed to parse Loki message: {}", err);
						}) else {
							break;
						};

						let logs = message
							.streams
							.into_iter()
							.flat_map(|stream| stream.values)
							.map(|(timestamp, log)| RunnerLog {
								timestamp: timestamp
									.parse::<i128>()
									.ok()
									.and_then(|ns| {
										OffsetDateTime::from_unix_timestamp_nanos(ns).ok()
									})
									.unwrap_or(OffsetDateTime::UNIX_EPOCH),
								log,
							})
							.collect();

						let Ok(()) = websocket
							.send(Message::Item(StreamRunnerLogsServerMsg::LogData { logs }))
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
