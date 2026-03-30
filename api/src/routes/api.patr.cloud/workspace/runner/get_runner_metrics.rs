use std::{fmt::Display, sync::OnceLock};

use axum::http::{HeaderName, HeaderValue, StatusCode};
use models::api::workspace::runner::*;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use crate::prelude::*;

/// A static reqwest client for querying Mimir
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MimirResponse {
	data: MimirData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MimirData {
	result: Vec<MimirMatrixResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MimirMatrixResult {
	values: Vec<(f64, String)>,
}

/// Route to get system metrics for a runner. Queries Mimir for CPU, memory,
/// disk, and network metrics using PromQL.
pub async fn get_runner_metrics(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetRunnerMetricsPath {
					workspace_id,
					runner_id,
				},
				query: GetRunnerMetricsQuery { interval },
				headers:
					GetRunnerMetricsRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetRunnerMetricsRequestProcessed,
			},
		database: _,
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, GetRunnerMetricsRequest>,
) -> Result<AppResponse<GetRunnerMetricsRequest>, ErrorType> {
	info!("Getting metrics for runner: {}", runner_id);

	let interval = interval.unwrap_or(Duration::hours(1));
	let step = rate_window_for_interval(interval).to_string();
	let now = OffsetDateTime::now_utc();
	let start = (now - interval).unix_timestamp().to_string();
	let end = now.unix_timestamp().to_string();
	let labels = format!("runner_id=\"{}\", source=\"runner\"", runner_id);
	let endpoint = state.config.opentelemetry.metrics.endpoint.clone();

	// Run all queries in parallel, but in separate tasks. Running them with a
	// join_all! causes a stack overflow due to the amount of stuff that's stored on
	// each query. task::spawn will allocate each task on the heap, so this avoids
	// the stack overflow.
	let (
		cpu_usage,
		memory_usage,
		disk_read_bytes,
		disk_written_bytes,
		disk_usage,
		network_usage_rx,
		network_usage_tx,
	) = (
		tokio::spawn(query_mimir(
			endpoint.clone(),
			workspace_id,
			format!(
				"100 - (avg(rate(node_cpu_seconds_total{{{}, mode=\"idle\"}}[{}])) * 100)",
				labels, step
			),
			start.clone(),
			end.clone(),
			step.clone(),
		)),
		tokio::spawn(query_mimir(
			endpoint.clone(),
			workspace_id,
			format!(
				concat!(
					"(1 - (node_memory_MemAvailable_bytes{{{}}} ",
					"/ node_memory_MemTotal_bytes{{{}}})) * 100"
				),
				labels, labels
			),
			start.clone(),
			end.clone(),
			step.clone(),
		)),
		tokio::spawn(query_mimir(
			endpoint.clone(),
			workspace_id,
			format!("rate(node_disk_read_bytes_total{{{}}}[{}])", labels, step),
			start.clone(),
			end.clone(),
			step.clone(),
		)),
		tokio::spawn(query_mimir(
			endpoint.clone(),
			workspace_id,
			format!(
				"rate(node_disk_written_bytes_total{{{}}}[{}])",
				labels, step
			),
			start.clone(),
			end.clone(),
			step.clone(),
		)),
		tokio::spawn(query_mimir(
			endpoint.clone(),
			workspace_id,
			format!(
				concat!(
					"(1 - (node_filesystem_avail_bytes{{{}, mountpoint=\"/\"}} ",
					"/ node_filesystem_size_bytes{{{}, mountpoint=\"/\"}})) * 100"
				),
				labels, labels
			),
			start.clone(),
			end.clone(),
			step.clone(),
		)),
		tokio::spawn(query_mimir(
			endpoint.clone(),
			workspace_id,
			format!(
				"rate(node_network_receive_bytes_total{{{}, device!=\"lo\"}}[{}])",
				labels, step
			),
			start.clone(),
			end.clone(),
			step.clone(),
		)),
		tokio::spawn(query_mimir(
			endpoint.clone(),
			workspace_id,
			format!(
				"rate(node_network_transmit_bytes_total{{{}, device!=\"lo\"}}[{}])",
				labels, step
			),
			start.clone(),
			end.clone(),
			step.clone(),
		)),
	);

	let (
		cpu_usage,
		memory_usage,
		disk_read_bytes,
		disk_written_bytes,
		disk_usage,
		network_usage_rx,
		network_usage_tx,
	) = (
		cpu_usage
			.await
			.map_err(|err| ErrorType::server_error(err.to_string()))??,
		memory_usage
			.await
			.map_err(|err| ErrorType::server_error(err.to_string()))??,
		disk_read_bytes
			.await
			.map_err(|err| ErrorType::server_error(err.to_string()))??,
		disk_written_bytes
			.await
			.map_err(|err| ErrorType::server_error(err.to_string()))??,
		disk_usage
			.await
			.map_err(|err| ErrorType::server_error(err.to_string()))??,
		network_usage_rx
			.await
			.map_err(|err| ErrorType::server_error(err.to_string()))??,
		network_usage_tx
			.await
			.map_err(|err| ErrorType::server_error(err.to_string()))??,
	);

	let metrics = RunnerMetrics {
		cpu_usage,
		memory_usage,
		disk_read_bytes,
		disk_written_bytes,
		disk_usage,
		network_usage_rx,
		network_usage_tx,
	};

	AppResponse::builder()
		.body(GetRunnerMetricsResponse { metrics })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}

/// Derive the rate window and step from the requested interval to keep data
/// point count reasonable.
fn rate_window_for_interval(interval: Duration) -> &'static str {
	if interval <= Duration::hours(1) {
		"2m"
	} else if interval <= Duration::hours(6) {
		"5m"
	} else if interval <= Duration::hours(24) {
		"15m"
	} else if interval <= Duration::days(7) {
		"1h"
	} else {
		"4h"
	}
}

/// Query Mimir for a single PromQL expression and return data points.
/// Takes owned values so it can be spawned as a tokio task.
async fn query_mimir(
	endpoint: impl Display,
	workspace_id: Uuid,
	query: impl Display,
	start: impl Display,
	end: impl Display,
	step: impl Display,
) -> Result<Vec<MetricDataPoint>, ErrorType> {
	let response = CLIENT
		.get_or_init(reqwest::Client::new)
		.get(format!("{}/prometheus/api/v1/query_range", endpoint))
		.query(&[
			("query", query.to_string()),
			("start", start.to_string()),
			("end", end.to_string()),
			("step", step.to_string()),
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
	}) = serde_json::from_str::<MimirResponse>(&response)
	else {
		warn!("Cannot parse Mimir response: {}", response);
		return Ok(Vec::new());
	};

	Ok(result
		.into_iter()
		.next()
		.map(|r| {
			r.values
				.into_iter()
				.map(|(ts, value)| MetricDataPoint {
					timestamp: OffsetDateTime::from_unix_timestamp(ts as i64)
						.unwrap_or(OffsetDateTime::UNIX_EPOCH),
					value,
				})
				.collect()
		})
		.unwrap_or_default())
}
