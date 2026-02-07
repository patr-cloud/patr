/// All handlers for authentication and authorization.
mod auth;
/// All handlers for user related data.
mod user;
/// All handlers for resources that would, in the managed version, be part of
/// the workspace.
mod workspace;

use std::sync::OnceLock;

use axum::{Router, body::Body, http::Request, response::Response};
use tokio::fs;
use tower_http::services::ServeFile;

use crate::prelude::*;

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

	if cfg!(debug_assertions) {
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
		router.merge(read_files("./frontend/dist").await.into_iter().fold(
			Router::new(),
			|router, file| {
				router.route_service(
					file.trim_start_matches("./frontend/dist"),
					ServeFile::new(file.as_str()),
				)
			},
		))
	}
}

/// Reads all files in a directory and its subdirectories
async fn read_files(path: &str) -> Vec<String> {
	let mut files = Vec::new();
	let mut read_dir = fs::read_dir(path)
		.await
		.unwrap_or_else(|_| panic!("failed to read directory: `{path}`"));
	while let Some(entry) = read_dir.next_entry().await.expect("failed to read entry") {
		let path = entry.path();
		if path.is_dir() {
			files.extend(Box::pin(read_files(path.to_str().unwrap())).await);
		} else {
			files.push(path.to_str().unwrap().to_string());
		}
	}
	files
}
