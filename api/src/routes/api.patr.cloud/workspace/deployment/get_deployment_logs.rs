use axum::http::{HeaderName, HeaderValue, StatusCode};
use models::api::workspace::deployment::*;
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

/// Route to get the logs of a deployment. This will fetch logs from Loki
/// and return them to the user. The logs can be filtered by time and search
/// query.
pub async fn get_deployment_logs(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetDeploymentLogsPath {
					workspace_id,
					deployment_id,
				},
				query: GetDeploymentLogsQueryProcessed {
					end_time,
					limit,
					search,
				},
				headers:
					GetDeploymentLogsRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetDeploymentLogsRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, GetDeploymentLogsRequest>,
) -> Result<AppResponse<GetDeploymentLogsRequest>, ErrorType> {
	info!("Getting logs for deployment: {}", deployment_id);

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

	let end = end_time.unwrap_or(OffsetDateTime::now_utc());
	let start = end - time::Duration::days(30);

	let loki_response = reqwest::Client::new()
		.get(format!(
			"{}/loki/api/v1/query_range",
			state.config.opentelemetry.logs.endpoint
		))
		.query(&[
			("limit", limit.unwrap_or(100).to_string()),
			("direction", "backward".to_string()),
			("start", start.unix_timestamp_nanos().to_string()),
			("end", end.unix_timestamp_nanos().to_string()),
			(
				"query",
				format!(
					"{{deployment_id=\"{}\"}}{}",
					deployment_id,
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

	let mut logs: Vec<DeploymentLog> = result
		.into_iter()
		.flat_map(|LokiMatrixResult { values }| {
			values.into_iter().map(|(timestamp, log)| DeploymentLog {
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
		.body(GetDeploymentLogsResponse { logs })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
