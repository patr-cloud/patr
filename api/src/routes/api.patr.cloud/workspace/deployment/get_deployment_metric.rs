use std::sync::OnceLock;

use axum::http::{HeaderName, HeaderValue, StatusCode};
use models::api::workspace::{MetricDataPoint, deployment::*};
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

/// Route to get a single metric for a deployment.
pub async fn get_deployment_metric(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					GetDeploymentMetricPath {
						workspace_id,
						deployment_id,
						metric,
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
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, GetDeploymentMetricRequest>,
) -> Result<AppResponse<GetDeploymentMetricRequest>, ErrorType> {
	info!(
		"Getting deployment metric `{}` for deployment: {}",
		metric, deployment_id
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

	let interval = interval.unwrap_or(Duration::hours(1));
	let step = rate_window_for_interval(interval).to_string();
	let now = OffsetDateTime::now_utc();
	let start = (now - interval).unix_timestamp().to_string();
	let end = now.unix_timestamp().to_string();
	let d = format!("deployment_id=\"{}\"", deployment_id);
	let endpoint = state.config.opentelemetry.metrics.endpoint.clone();

	let query = match metric {
		DeploymentMetricName::IngressRps => {
			format!("sum(rate(patr_ingress_requests_total{{{}}}[{}]))", d, step)
		}
		DeploymentMetricName::IngressLatencyP50 => format!(
			"histogram_quantile(0.5, sum(rate(patr_ingress_request_duration_seconds_bucket{{{}}}[{}])) by (le))",
			d, step
		),
		DeploymentMetricName::IngressLatencyP95 => format!(
			"histogram_quantile(0.95, sum(rate(patr_ingress_request_duration_seconds_bucket{{{}}}[{}])) by (le))",
			d, step
		),
		DeploymentMetricName::IngressLatencyP99 => format!(
			"histogram_quantile(0.99, sum(rate(patr_ingress_request_duration_seconds_bucket{{{}}}[{}])) by (le))",
			d, step
		),
		DeploymentMetricName::IngressTtfbP50 => format!(
			"histogram_quantile(0.5, sum(rate(patr_ingress_response_duration_seconds_bucket{{{}}}[{}])) by (le))",
			d, step
		),
		DeploymentMetricName::IngressTtfbP95 => format!(
			"histogram_quantile(0.95, sum(rate(patr_ingress_response_duration_seconds_bucket{{{}}}[{}])) by (le))",
			d, step
		),
		DeploymentMetricName::IngressTtfbP99 => format!(
			"histogram_quantile(0.99, sum(rate(patr_ingress_response_duration_seconds_bucket{{{}}}[{}])) by (le))",
			d, step
		),
		DeploymentMetricName::IngressErrorRate => {
			format!(
				"sum(rate(patr_ingress_request_errors_total{{{}}}[{}]))",
				d, step
			)
		}
		DeploymentMetricName::IngressStatus2xx => format!(
			"sum(rate(patr_ingress_requests_total{{{}, code=~\"2..\"}}[{}]))",
			d, step
		),
		DeploymentMetricName::IngressStatus3xx => format!(
			"sum(rate(patr_ingress_requests_total{{{}, code=~\"3..\"}}[{}]))",
			d, step
		),
		DeploymentMetricName::IngressStatus4xx => format!(
			"sum(rate(patr_ingress_requests_total{{{}, code=~\"4..\"}}[{}]))",
			d, step
		),
		DeploymentMetricName::IngressStatus5xx => format!(
			"sum(rate(patr_ingress_requests_total{{{}, code=~\"5..\"}}[{}]))",
			d, step
		),
		DeploymentMetricName::IngressBandwidthIn => format!(
			"sum(rate(patr_ingress_request_size_bytes_sum{{{}}}[{}]))",
			d, step
		),
		DeploymentMetricName::IngressBandwidthOut => format!(
			"sum(rate(patr_ingress_response_size_bytes_sum{{{}}}[{}]))",
			d, step
		),
		DeploymentMetricName::IngressActiveConnections => {
			format!("sum(patr_ingress_requests_in_flight{{{}}})", d)
		}
		DeploymentMetricName::IngressRequestBodySize => format!(
			concat!(
				"sum(rate(patr_ingress_request_size_bytes_sum{{{}}}[{}]))",
				" / ",
				"sum(rate(patr_ingress_request_size_bytes_count{{{}}}[{}]))"
			),
			d, step, d, step
		),
		DeploymentMetricName::IngressResponseBodySize => format!(
			concat!(
				"sum(rate(patr_ingress_response_size_bytes_sum{{{}}}[{}]))",
				" / ",
				"sum(rate(patr_ingress_response_size_bytes_count{{{}}}[{}]))"
			),
			d, step, d, step
		),
		DeploymentMetricName::ContainerCpuUsage => format!(
			"rate(patr_container_cpu_usage_seconds_total{{{}}}[{}]) * 100",
			d, step
		),
		DeploymentMetricName::ContainerCpuThrottling => format!(
			"rate(patr_container_cpu_throttled_seconds_total{{{}}}[{}])",
			d, step
		),
		DeploymentMetricName::ContainerMemoryUsed => {
			format!("patr_container_memory_used_bytes{{{}}}", d)
		}
		DeploymentMetricName::ContainerMemoryLimit => {
			format!("patr_container_memory_limit_bytes{{{}}}", d)
		}
		DeploymentMetricName::ContainerNetworkRx => format!(
			"rate(patr_container_network_rx_bytes_total{{{}}}[{}])",
			d, step
		),
		DeploymentMetricName::ContainerNetworkTx => format!(
			"rate(patr_container_network_tx_bytes_total{{{}}}[{}])",
			d, step
		),
		DeploymentMetricName::ContainerDiskRead => format!(
			"rate(patr_container_disk_read_bytes_total{{{}}}[{}])",
			d, step
		),
		DeploymentMetricName::ContainerDiskWrite => format!(
			"rate(patr_container_disk_write_bytes_total{{{}}}[{}])",
			d, step
		),
		DeploymentMetricName::ContainerOomKills => {
			format!("patr_container_oom_kills_total{{{}}}", d)
		}
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
		.body(GetDeploymentMetricResponse { data_points })
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
