use std::sync::OnceLock;

use axum::body::Body;
use http::StatusCode;
use rustis::client::Client as RedisClient;

use crate::prelude::*;

/// A static reqwest client for proxying requests to the upstream Loki instance
#[doc(hidden)]
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Parse a Prometheus-style label string like `{key="value", key2="value2"}`
/// into key-value pairs.
pub(super) fn parse_labels(labels: &str) -> Vec<(String, String)> {
	let trimmed = labels.trim();
	let inner = trimmed
		.strip_prefix('{')
		.and_then(|s| s.strip_suffix('}'))
		.unwrap_or(trimmed);

	if inner.is_empty() {
		return Vec::new();
	}

	let mut result = Vec::new();
	for pair in inner.split(',') {
		let pair = pair.trim();
		if let Some((key, rest)) = pair.split_once('=') {
			let value = rest
				.trim()
				.strip_prefix('"')
				.and_then(|s| s.strip_suffix('"'))
				.unwrap_or(rest.trim());
			result.push((key.trim().to_string(), value.to_string()));
		}
	}
	result
}

/// Serialize label pairs back into Prometheus format: `{key="value", ...}`
pub(super) fn serialize_labels(labels: &[(String, String)]) -> String {
	let inner: Vec<String> = labels.iter().map(|(k, v)| format!("{k}=\"{v}\"")).collect();
	format!("{{{}}}", inner.join(", "))
}

/// Validate and rewrite labels for a single stream. Returns an error message
/// if validation fails.
pub(super) async fn validate_and_rewrite_labels(
	database: &mut DatabaseConnection,
	redis: &mut RedisClient,
	labels: &str,
	runner_id: &Uuid,
	workspace_id: &Uuid,
) -> Result<String, String> {
	let mut pairs = parse_labels(labels);

	// Find and validate deployment_id
	let deployment_id_value = pairs
		.iter()
		.find(|(k, _)| k == "deployment_id")
		.map(|(_, v)| v.clone());

	if let Some(ref dep_id_str) = deployment_id_value {
		let deployment_id: Uuid = dep_id_str
			.parse()
			.map_err(|_| format!("Invalid deployment_id: {dep_id_str}"))?;

		let owning_runner =
			super::cache::get_runner_for_deployment(database, redis, &deployment_id)
				.await
				.map_err(|e| format!("DB error looking up deployment: {e}"))?
				.ok_or_else(|| format!("Deployment {deployment_id} not found"))?;

		if owning_runner != *runner_id {
			return Err(format!(
				"Deployment {deployment_id} does not belong to runner {runner_id}"
			));
		}
	}

	// Upsert runner_id and workspace_id with server-derived values
	if let Some((_, v)) = pairs.iter_mut().find(|(k, _)| k == "runner_id") {
		*v = runner_id.to_string();
	} else {
		pairs.push(("runner_id".to_string(), runner_id.to_string()));
	}
	if let Some((_, v)) = pairs.iter_mut().find(|(k, _)| k == "workspace_id") {
		*v = workspace_id.to_string();
	} else {
		pairs.push(("workspace_id".to_string(), workspace_id.to_string()));
	}

	// Force-set source label based on deployment_id presence
	let source_value = if deployment_id_value.is_some() {
		"deployment"
	} else {
		"runner"
	};
	if let Some((_, v)) = pairs.iter_mut().find(|(k, _)| k == "source") {
		*v = source_value.to_string();
	} else {
		pairs.push(("source".to_string(), source_value.to_string()));
	}

	Ok(serialize_labels(&pairs))
}

/// Forward an encoded payload to the upstream Loki instance.
pub(super) async fn forward_to_loki(
	state: &AppState,
	method: &http::Method,
	path: &str,
	original_headers: &http::HeaderMap,
	workspace_id: &Uuid,
	body: Vec<u8>,
) -> axum::response::Response {
	let upstream_base = &state.config.opentelemetry.logs.endpoint;
	let upstream_url = format!("{}{}", upstream_base, path);

	let mut forwarded_headers = original_headers.clone();
	forwarded_headers.remove(http::header::HOST);
	forwarded_headers.remove(http::header::AUTHORIZATION);
	forwarded_headers.remove(http::header::CONTENT_LENGTH);
	forwarded_headers.remove("X-Scope-OrgID");

	let Ok(response) = CLIENT
		.get_or_init(reqwest::Client::new)
		.request(method.clone(), &upstream_url)
		.headers(forwarded_headers)
		.header("X-Scope-OrgID", workspace_id.to_string())
		.body(body)
		.send()
		.await
		.inspect_err(|err| {
			error!("Error proxying request to Loki: {}", err);
		})
	else {
		return axum::response::Response::builder()
			.status(StatusCode::BAD_GATEWAY)
			.body(Body::from("Bad Gateway"))
			.unwrap();
	};

	let status = response.status();
	let headers = response.headers().clone();
	let body = response.bytes_stream();

	let mut resp = axum::response::Response::builder().status(status);
	for (key, value) in headers.iter() {
		if key != "transfer-encoding" {
			resp = resp.header(key, value);
		}
	}

	resp.body(Body::from_stream(body)).unwrap()
}
