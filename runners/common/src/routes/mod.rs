/// All handlers for authentication and authorization.
pub mod auth;
/// All handlers for resources that would, in the managed version, be part of
/// the workspace.
pub mod workspace;

use axum::Router;
use leptos::*;
use leptos_axum::LeptosRoutes;
use tokio::fs;
use tower_http::services::ServeFile;

use crate::prelude::*;

/// Sets up the routes for the API, across all domains.
#[instrument(skip(state))]
pub async fn setup_routes<E>(state: &AppState<E>) -> Router
where
	E: RunnerExecutor + Clone + 'static,
{
	let config = leptos::get_configuration(
		if option_env!("LEPTOS_OUTPUT_NAME").is_some() {
			None
		} else {
			Some(concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.toml"))
		},
	)
	.await
	.expect("failed to get configuration");

	read_files(&config.leptos_options.site_root)
		.await
		.into_iter()
		.fold(Router::new(), |router, file| {
			router.route_service(
				file.trim_start_matches(config.leptos_options.site_root.as_str()),
				ServeFile::new(file.as_str()),
			)
		})
		.leptos_routes(
			&config.leptos_options,
			leptos_axum::generate_route_list(frontend::render),
			frontend::render,
		)
		.with_state(config.leptos_options)
		.with_state(state.clone())
		.merge(
			if cfg!(debug_assertions) {
				Router::new().nest(
					"/api2",
					Router::new()
						.merge(workspace::setup_routes(state).await)
						.merge(auth::setup_routes(state).await),
				)
			} else {
				Router::new()
			},
		)
}

/// Reads all files in a directory and its subdirectories
async fn read_files(path: &str) -> Vec<String> {
	let mut files = Vec::new();
	let mut read_dir = fs::read_dir(path)
		.await
		.unwrap_or_else(|_| panic!("failed to read directory: `{}`", path));
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
