use axum::http::StatusCode;
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
					workspace_id: _,
					deployment_id,
				},
				query: GetDeploymentLogsQuery {
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
		config,
		user_data: _,
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

	let loki_response = reqwest::Client::new()
		.get(format!("{}/loki/api/v1/query_range", config.loki.endpoint))
		.basic_auth(config.loki.username, Some(config.loki.password))
		.query(&{
			let mut query = vec![
				("limit", limit.unwrap_or(100).to_string()),
				(
					"end",
					end_time
						.unwrap_or(OffsetDateTime::now_utc())
						.unix_timestamp_nanos()
						.to_string(),
				),
			];

			if let Some(search) = search {
				query.extend([("query", format!("{{}} |= \"{}\"", search))]);
			}

			query
		})
		.send()
		.await?
		.text()
		.await?;

	let Ok(LokiResponse {
		data: LokiData {
			result: [LokiMatrixResult { values }],
		},
	}) = serde_json::from_str::<LokiResponse>(&loki_response)
	else {
		error!("Cannot parse Loki response: {}", loki_response);
		return Err(ErrorType::server_error(format!(
			"Failed to parse Loki response"
		)));
	};

	let logs = values
		.into_iter()
		.map(|(timestamp, log)| DeploymentLog {
			timestamp: OffsetDateTime::from_unix_timestamp_nanos(timestamp)
				.unwrap_or(OffsetDateTime::UNIX_EPOCH),
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
