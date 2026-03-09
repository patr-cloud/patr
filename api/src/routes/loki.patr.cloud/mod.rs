use std::sync::OnceLock;

use axum::{
	Router,
	body::Body,
	extract::{ConnectInfo, State},
	http::Request,
	response::Response,
	routing::post,
};
use base64::prelude::*;
use http::StatusCode;

use crate::{models::permissions, prelude::*};

/// A static reqwest client for proxying requests to the upstream Loki instance
#[doc(hidden)]
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Sets up the routes for loki.patr.cloud
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.route("/loki/api/v1/push", post(proxy_to_loki))
		.route("/otlp/v1/logs", post(proxy_to_loki))
		.with_state(state.clone())
}

/// Extract workspace ID and API token from HTTP Basic Auth header.
/// Returns `(workspace_id, api_token)` or None if invalid.
fn extract_basic_auth(req: &Request<Body>) -> Option<(Uuid, String)> {
	let auth_header = req.headers().get(http::header::AUTHORIZATION)?;
	let auth_str = auth_header.to_str().ok()?;
	let encoded = auth_str.strip_prefix("Basic ")?;
	let decoded = String::from_utf8(BASE64_STANDARD.decode(encoded).ok()?).ok()?;
	let (workspace_id_str, api_token) = decoded.split_once(':')?;
	let workspace_id = workspace_id_str.parse::<Uuid>().ok()?;
	Some((workspace_id, api_token.to_string()))
}

/// Handler that authenticates the request and proxies it to the upstream Loki
/// instance with streaming.
async fn proxy_to_loki(
	State(state): State<AppState>,
	ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
	req: Request<Body>,
) -> Response {
	// Extract Basic Auth credentials
	let Some((workspace_id, api_token)) = extract_basic_auth(&req) else {
		return Response::builder()
			.status(StatusCode::UNAUTHORIZED)
			.header("WWW-Authenticate", "Basic realm=\"Patr Loki\"")
			.body(Body::from("Missing or invalid Authorization header"))
			.unwrap();
	};

	// Validate the API token and get user permissions
	let mut database = match state.database.acquire().await {
		Ok(conn) => conn,
		Err(err) => {
			error!("Failed to acquire database connection: {}", err);
			return Response::builder()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.body(Body::from("Internal Server Error"))
				.unwrap();
		}
	};
	let mut redis = state.redis.clone();

	let user_data = match permissions::get_user_data_for_token(
		&mut *database,
		&mut redis,
		ClientType::ApiToken,
		&state.config,
		addr.ip(),
		&api_token,
	)
	.await
	{
		Ok(data) => data,
		Err(err) => {
			warn!("Authentication failed: {}", err);
			return Response::builder()
				.status(StatusCode::UNAUTHORIZED)
				.header("WWW-Authenticate", "Basic realm=\"Patr Loki\"")
				.body(Body::from("Authentication failed"))
				.unwrap();
		}
	};

	// TODO Verify the user has access to push here
	if !user_data.permissions.contains_key(&workspace_id) {
		warn!(
			"User {} does not have access to workspace {}",
			user_data.id, workspace_id
		);
		return Response::builder()
			.status(StatusCode::FORBIDDEN)
			.body(Body::from("Access denied for this workspace"))
			.unwrap();
	}

	// Build the upstream URL
	let upstream_base = &state.config.opentelemetry.logs.endpoint;
	let request_path = req.uri().path();
	let upstream_url = format!("{}{}", upstream_base, request_path);

	// Proxy the request to the upstream Loki instance
	let Ok(response) = CLIENT
		.get_or_init(reqwest::Client::new)
		.request(req.method().clone(), &upstream_url)
		.headers({
			let mut headers = req.headers().clone();
			headers.remove(http::header::HOST);
			headers.remove(http::header::AUTHORIZATION);
			headers
		})
		.header("X-Scope-OrgID", workspace_id.to_string())
		.body(reqwest::Body::wrap_stream(
			req.into_body().into_data_stream(),
		))
		.send()
		.await
		.inspect_err(|err| {
			error!("Error proxying request to Loki: {}", err);
		})
	else {
		return Response::builder()
			.status(StatusCode::BAD_GATEWAY)
			.body(Body::from("Bad Gateway"))
			.unwrap();
	};

	// Stream the response back
	let status = response.status();
	let headers = response.headers().clone();
	let body = response.bytes_stream();

	let mut response = Response::builder().status(status);

	for (key, value) in headers.iter() {
		if key != "transfer-encoding" {
			response = response.header(key, value);
		}
	}

	response.body(Body::from_stream(body)).unwrap()
}
