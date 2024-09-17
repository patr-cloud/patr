use axum::http::StatusCode;
use models::api::workspace::deployment::*;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LokiResponse {
	data: LokiData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LokiData {
	result: [LokiMatrixResult; 1],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LokiMatrixResult {
	values: Vec<(i128, String)>,
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
				query: GetDeploymentLogsQuery {
					end_time,
					limit,
					search,
				},
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

	let Some(loki) = config.loki else {
		return Err(ErrorType::server_error("Loki configuration not found"));
	};
	let loki_response = reqwest::Client::new()
		.get(format!("{}/loki/api/v1/query_range", loki.endpoint))
		.basic_auth(loki.username, Some(loki.password))
		.query(&{
			let mut query = vec![
				("limit", limit.unwrap_or(100)),
				(
					"end",
					end_time
						.unwrap_or(OffsetDateTime::now_utc())
						.unix_timestamp_nanos()
						.to_string(),
				),
			];

			if let Some(search) = search {
				query.extend_one(("query", format!("{{}} |= \"{}\"", search)));
			}

			query
		})
		.send()
		.await?
		.text()
		.await?;

	let Ok(loki_response) = serde_json::from_str::<LokiResponse>(&loki_response) else {
		error!("Cannot parse Loki response: {}", loki_response);
		return Err(ErrorType::server_error(format!(
			"Failed to parse Loki response"
		)));
	};

	let logs = loki_response.data.result[0]
		.values
		.into_iter()
		.map(|(timestamp, log)| DeploymentLog {
			timestamp: OffsetDateTime::from_unix_timestamp_nanos(timestamp),
			log,
		})
		.collect();

	AppResponse::builder()
		.body(GetDeploymentLogsResponse { logs })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
