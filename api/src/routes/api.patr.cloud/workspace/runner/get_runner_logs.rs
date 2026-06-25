use axum::http::{HeaderName, HeaderValue, StatusCode};
use models::api::workspace::runner::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LokiResponse {
	data: LokiData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LokiData {
	result: Vec<LokiMatrixResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LokiMatrixResult {
	values: Vec<(String, String)>,
}

/// Route to get the logs of a runner process. This will fetch logs from Loki
/// filtered by runner_id and source="runner".
pub async fn get_runner_logs(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetRunnerLogsPath {
					workspace_id,
					runner_id,
				},
				query: GetRunnerLogsQueryProcessed {
					end_time,
					limit,
					search,
				},
				headers:
					GetRunnerLogsRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetRunnerLogsRequestProcessed,
			},
		database: _,
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, GetRunnerLogsRequest>,
) -> Result<AppResponse<GetRunnerLogsRequest>, ErrorType> {
	info!("Getting logs for runner: {}", runner_id);

	// Runner existence is already validated by ResourcePermissionAuthenticator

	let loki_response = reqwest::Client::new()
		.get(format!(
			"{}/loki/api/v1/query_range",
			state.config.opentelemetry.logs.endpoint
		))
		.query(&[
			("limit", limit.unwrap_or(100).to_string()),
			("direction", "backward".to_string()),
			(
				"end",
				end_time
					.unwrap_or(OffsetDateTime::now_utc())
					.unix_timestamp_nanos()
					.to_string(),
			),
			(
				"query",
				format!(
					"{{runner_id=\"{}\", source=\"runner\"}}{}",
					runner_id,
					search
						.map(|search| format!(" |= `{}`", search))
						.unwrap_or_default()
				),
			),
		])
		.header(
			HeaderName::from_static("x-scope-orgid"),
			HeaderValue::from_str(&workspace_id.to_string()).unwrap(),
		)
		.send()
		.await?
		.text()
		.await?;

	trace!("{}", &loki_response);

	let Ok(LokiResponse {
		data: LokiData { result },
	}) = serde_json::from_str::<LokiResponse>(&loki_response)
	else {
		error!("Cannot parse Loki response: {}", loki_response);
		return Err(ErrorType::server_error("Failed to parse Loki response"));
	};

	let mut logs: Vec<RunnerLog> = result
		.into_iter()
		.flat_map(|LokiMatrixResult { values }| {
			values.into_iter().map(|(timestamp, log)| RunnerLog {
				timestamp: timestamp
					.parse::<i128>()
					.ok()
					.and_then(|ns| OffsetDateTime::from_unix_timestamp_nanos(ns).ok())
					.unwrap_or(OffsetDateTime::UNIX_EPOCH),
				log,
			})
		})
		.collect();

	// Sort ascending (oldest first, newest last) — the frontend renders
	// index 0 at the top and auto-scrolls to the bottom (latest entry)
	logs.sort_by_key(|a| a.timestamp);

	AppResponse::builder()
		.body(GetRunnerLogsResponse { logs })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
