use std::{sync::OnceLock, time::Duration};

#[cfg(feature = "cloud")]
use axum::Router;
use axum::{body::Body, http::Request, response::Response};
#[cfg(feature = "cloud")]
use http::StatusCode;
#[cfg(feature = "cloud")]
use models::{ApiErrorResponse, ApiErrorResponseBody, utils::False};

use crate::prelude::*;
#[cfg(feature = "cloud")]
use crate::routes::api_patr_cloud;

/// A static reqwest client for proxying requests to the frontend server.
#[doc(hidden)]
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Sets up the routes for the web dashboard
#[cfg(feature = "cloud")]
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.with_state(state.clone())
		.nest(
			"/api",
			api_patr_cloud::setup_routes(
				state,
				&[ActorClientType::UserLogin(UserLoginType::WebLogin)],
			)
			.await
			.fallback(async |req: Request<Body>| ApiErrorResponse {
				status_code: StatusCode::NOT_FOUND,
				body: ApiErrorResponseBody {
					success: False,
					error: ErrorType::WrongParameters,
					message: format!("No API route found for {}", req.uri().path()),
				},
			}),
		)
		.fallback(proxy)
}

/// Reverse-proxies the given request to the frontend Node server. The
/// frontend's base URL is read from the `FRONTEND_URL` env var (default
/// `http://localhost:3030`).
#[axum::debug_handler]
pub async fn proxy(req: Request<Body>) -> Response {
	let Ok(response) = CLIENT
		.get_or_init(|| {
			// Node closes a keep-alive connection after 5 idle seconds. Dropping
			// ours sooner keeps a request from going out on a connection Node has
			// just closed, which fails it with a 502.
			reqwest::Client::builder()
				.pool_idle_timeout(Duration::from_secs(4))
				.build()
				.expect("failed to build the frontend proxy client")
		})
		.request(
			req.method().clone(),
			format!(
				"{}{}",
				std::env::var("FRONTEND_URL")
					.unwrap_or_else(|_| "http://localhost:3030".to_string()),
				req.uri()
					.path_and_query()
					.map(|v| v.as_str())
					.unwrap_or_default()
			),
		)
		.headers(req.headers().clone())
		.body(reqwest::Body::wrap_stream(
			req.into_body().into_data_stream(),
		))
		.send()
		.await
		.inspect_err(|err| {
			error!("Error proxying request to frontend: {}", err);
		})
	else {
		return Response::builder()
			.status(502)
			.body(Body::from("Bad Gateway"))
			.unwrap();
	};

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
