use std::sync::OnceLock;

use axum::http::{HeaderName, HeaderValue, StatusCode};
use models::api::workspace::{MetricDataPoint, runner::*};
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

/// Route to get a single system metric for a runner.
pub async fn get_runner_metrics(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: GetRunnerMetricsPath {
					workspace_id,
					runner_id,
					metric,
				},
				query: GetRunnerMetricsQueryProcessed { interval },
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
	info!("Getting metric `{}` for runner: {}", metric, runner_id);

	let interval = interval.unwrap_or(Duration::hours(1));
	let step = rate_window_for_interval(interval).to_string();
	let now = OffsetDateTime::now_utc();
	let start = (now - interval).unix_timestamp().to_string();
	let end = now.unix_timestamp().to_string();
	let labels = format!("runner_id=\"{}\", source=\"runner\"", runner_id);
	let endpoint = state.config.opentelemetry.metrics.endpoint.clone();

	let query = match metric {
		RunnerMetricName::SystemCpuUsage => format!(
			"100 - (avg(rate(node_cpu_seconds_total{{{}, mode=\"idle\"}}[{}])) * 100)",
			labels, step
		),
		RunnerMetricName::SystemMemoryUsage => format!(
			concat!(
				"(1 - (node_memory_MemAvailable_bytes{{{}}} ",
				"/ node_memory_MemTotal_bytes{{{}}})) * 100"
			),
			labels, labels
		),
		RunnerMetricName::SystemDiskReadBytes => {
			format!("rate(node_disk_read_bytes_total{{{}}}[{}])", labels, step)
		}
		RunnerMetricName::SystemDiskWrittenBytes => format!(
			"rate(node_disk_written_bytes_total{{{}}}[{}])",
			labels, step
		),
		RunnerMetricName::SystemDiskUsage => format!(
			concat!(
				"(1 - (node_filesystem_avail_bytes{{{}, mountpoint=\"/\"}} ",
				"/ node_filesystem_size_bytes{{{}, mountpoint=\"/\"}})) * 100"
			),
			labels, labels
		),
		RunnerMetricName::SystemNetworkRx => format!(
			"rate(node_network_receive_bytes_total{{{}, device!=\"lo\"}}[{}])",
			labels, step
		),
		RunnerMetricName::SystemNetworkTx => format!(
			"rate(node_network_transmit_bytes_total{{{}, device!=\"lo\"}}[{}])",
			labels, step
		),
	};

	let response = CLIENT
		.get_or_init(reqwest::Client::new)
		.get(format!("{}/prometheus/api/v1/query_range", endpoint))
		.query(&[
			("query", query),
			("start", start),
			("end", end),
			("step", step),
		])
		.header(
			HeaderName::from_static("x-scope-orgid"),
			HeaderValue::from_str(&workspace_id.to_string()).unwrap(),
		)
		.send()
		.await?
		.text()
		.await?;

	let MimirResponse {
		data: MimirData { result },
	} = serde_json::from_str::<MimirResponse>(&response).map_err(|err| {
		error!("Cannot parse Mimir response: {}", response);
		ErrorType::server_error(err.to_string())
	})?;

	let data_points = result
		.into_iter()
		.flat_map(|r| r.values)
		.map(|(ts, value)| MetricDataPoint {
			timestamp: OffsetDateTime::from_unix_timestamp(ts as i64)
				.unwrap_or(OffsetDateTime::UNIX_EPOCH),
			value,
		})
		.collect();

	AppResponse::builder()
		.body(GetRunnerMetricsResponse { data_points })
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
