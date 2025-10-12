use axum::{Router, body::Body, http::Request, response::Response, routing::get};

use crate::{prelude::*, routes::api_patr_cloud};

/// Sets up the routes for the web dashboard
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.with_state(state.clone())
		.nest(
			"/api",
			api_patr_cloud::setup_routes(state, ClientType::WebDashboard).await,
		)
		.route("/{*any}", get(proxy))
}

#[axum::debug_handler]
async fn proxy(req: Request<Body>) -> Response {
	let Ok(response) = reqwest::Client::new()
		.get(format!(
			"http://localhost:3030{}",
			req.uri()
				.path_and_query()
				.map(|v| v.as_str())
				.unwrap_or_default()
		))
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

	let mut builder = Response::builder().status(response.status());

	for (key, value) in response.headers() {
		builder = builder.header(key, value);
	}

	let Ok(body) = response.bytes().await.inspect_err(|err| {
		error!("Error reading response body from frontend: {}", err);
	}) else {
		return Response::builder()
			.status(502)
			.body(Body::from("Bad Gateway"))
			.unwrap();
	};
	builder.body(Body::from(body)).unwrap()
}
