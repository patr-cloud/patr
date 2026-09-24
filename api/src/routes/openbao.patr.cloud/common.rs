use std::sync::OnceLock;

use axum::body::Body;
use http::StatusCode;

use crate::prelude::*;

/// A static reqwest client for proxying requests to the upstream OpenBao
/// instance
#[doc(hidden)]
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Forward a read to the upstream OpenBao instance, and stream its response
/// back unchanged so that any OpenBao-compatible client understands it.
///
/// The upstream path is built here from ids the caller has already been
/// authorized for — never from anything they sent — because the configured
/// token can read every path in OpenBao.
pub(super) async fn forward_to_openbao(
	state: &AppState,
	workspace_id: &Uuid,
	secret_id: &Uuid,
) -> axum::response::Response {
	let upstream_url = format!(
		"{}/v1/secret/data/{}/{}",
		state.config.open_bao.endpoint.trim_end_matches('/'),
		workspace_id,
		secret_id
	);

	let Ok(response) = CLIENT
		.get_or_init(reqwest::Client::new)
		.get(&upstream_url)
		.header("X-Vault-Token", &state.config.open_bao.token)
		.send()
		.await
		.inspect_err(|err| {
			error!("Error proxying request to OpenBao: {}", err);
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
