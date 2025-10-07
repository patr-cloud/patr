/// All handlers for authentication and authorization.
mod auth;
/// All handlers for user related data.
mod user;
/// All handlers for resources that would, in the managed version, be part of
/// the workspace.
mod workspace;

use axum::Router;
use tokio::fs;
use tower_http::services::ServeFile;

use crate::prelude::*;

/// Sets up the routes for the entire application
#[instrument(skip(state))]
pub async fn setup_routes<E>(state: &AppState<E>) -> Router
where
	E: RunnerExecutor + Send + 'static,
{
	read_files("./frontend/dist")
		.await
		.into_iter()
		.fold(Router::new(), |router, file| {
			router.route_service(
				file.trim_start_matches("./frontend/dist"),
				ServeFile::new(file.as_str()),
			)
		})
		.with_state(state.clone())
		.merge(auth::setup_routes(state).await)
		.merge(user::setup_routes(state).await)
		.merge(workspace::setup_routes(state).await)
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
