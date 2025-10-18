use std::sync::OnceLock;

use axum::{Router, body::Body, http::Request, response::Response};

use crate::{prelude::*, routes::api_patr_cloud};

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Sets up the routes for the web dashboard
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.with_state(state.clone())
		.nest(
			"/api",
			api_patr_cloud::setup_routes(state, ClientType::WebDashboard).await,
		)
		.fallback(proxy)
}

#[axum::debug_handler]
async fn proxy(req: Request<Body>) -> Response {
	let Ok(response) = CLIENT
		.get_or_init(|| reqwest::Client::new())
		.request(
			req.method().clone(),
			format!(
				"http://localhost:3030{}",
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
