/// All handlers for authentication and authorization.
mod auth;
/// All handlers for user related data.
mod user;
/// All handlers for resources that would, in the managed version, be part of
/// the workspace.
mod workspace;

use std::sync::OnceLock;

use axum::{
	Router,
	body::Body,
	http::{Request, header},
	response::{IntoResponse, Response},
};

use crate::prelude::*;

/// The embedded frontend assets for production mode. In production, these will
/// be served by the frontend server, so we don't need to include them in the
/// binary.
#[doc(hidden)]
#[derive(Debug, Clone, rust_embed::RustEmbed)]
#[folder = "../../frontend/.output/public"]
struct FrontendAssets;

/// A static reqwest client for proxying requests in debug mode
#[doc(hidden)]
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Sets up the routes for the entire application
#[instrument(skip(state))]
pub async fn setup_routes<E>(state: &AppState<E>) -> Router
where
	E: RunnerExecutor + Send + 'static,
{
	let router = Router::new()
		.merge(auth::setup_routes(state).await)
		.merge(user::setup_routes(state).await)
		.merge(workspace::setup_routes(state).await);

	if !cfg!(debug_assertions) {
		router.fallback(async |req: Request<Body>| {
			let Ok(response) = CLIENT
				.get_or_init(reqwest::Client::new)
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
		})
	} else {
		router.fallback(async |req: Request<Body>| {
			let path = req.uri().path().trim_start_matches('/');

			// Try the exact path, then fall back to index.html for SPA routing
			let file = FrontendAssets::get(path).or_else(|| FrontendAssets::get("index.html"));

			match file {
				Some(file) => {
					let mime = file.metadata.mimetype();
					([(header::CONTENT_TYPE, mime)], file.data).into_response()
				}
				None => Response::builder()
					.status(404)
					.body(Body::from("Not Found"))
					.unwrap(),
			}
		})
	}
}
