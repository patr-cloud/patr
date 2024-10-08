use axum::http::{HeaderName, HeaderValue, StatusCode};
use models::api::workspace::deployment::*;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MimirResponse {
	data: MimirData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MimirData {
	result: Option<[MimirMatrixResult; 1]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MimirMatrixResult {
	#[serde(rename = "value")]
	values: Vec<(i128, String)>,
}

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
				query: GetDeploymentMetricQuery { interval },
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

	let mimir_response = reqwest::Client::new()
		.get(format!(
			"{}/mimir/api/v1/query_range",
			config.opentelemetry.logs.endpoint
		))
		.query(&[
			(
				"start",
				OffsetDateTime::now_utc().unix_timestamp_nanos().to_string(),
			),
			(
				"end",
				(OffsetDateTime::now_utc() - interval.unwrap_or(Duration::hours(1)))
					.unix_timestamp_nanos()
					.to_string(),
			),
			("query", format!("{{deployment_id=\"{}\"}}", deployment_id)),
		])
		.header(
			HeaderName::from_static("x-scope-orgid"),
			HeaderValue::from_str(&workspace_id.to_string()).unwrap(),
		)
		.send()
		.await?
		.text()
		.await?;

	let Ok(MimirResponse {
		data: MimirData { result },
	}) = serde_json::from_str::<MimirResponse>(&mimir_response)
	else {
		error!("Cannot parse Mimir response: {}", mimir_response);
		return Err(ErrorType::server_error(format!(
			"Failed to parse Mimir response"
		)));
	};

	let metrics = result
		.map(|[MimirMatrixResult { values }]| {
			values
				.into_iter()
				.map(|(timestamp, metric)| DeploymentMetric {
					timestamp: OffsetDateTime::from_unix_timestamp_nanos(timestamp)
						.unwrap_or(OffsetDateTime::UNIX_EPOCH),
					cpu_usage: String::new(),
					memory_usage: String::new(),
					network_usage_tx: String::new(),
					network_usage_rx: String::new(),
				})
				.collect()
		})
		.unwrap_or_default();

	AppResponse::builder()
		.body(GetDeploymentMetricResponse { metrics })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
