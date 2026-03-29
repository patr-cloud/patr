use std::sync::OnceLock;

use axum::body::Body;
use http::StatusCode;

use crate::prelude::*;

/// A static reqwest client for proxying requests to the upstream Mimir instance
#[doc(hidden)]
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Forward an encoded payload to the upstream Mimir instance.
pub(super) async fn forward_to_mimir(
	state: &AppState,
	method: &http::Method,
	path: &str,
	original_headers: &http::HeaderMap,
	workspace_id: &Uuid,
	body: Vec<u8>,
) -> axum::response::Response {
	let upstream_base = &state.config.opentelemetry.metrics.endpoint;
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
			error!("Error proxying request to Mimir: {}", err);
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
