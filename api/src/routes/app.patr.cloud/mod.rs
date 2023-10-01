use std::{future::Future, pin::Pin};

use axum::{routing::post, Router};
use leptos_axum::LeptosRoutes;
use tokio::fs;
use tower_http::services::ServeFile;

use crate::prelude::*;

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	let config = leptos::get_configuration(None)
		.await
		.expect("failed to get configuration");

	let mut router = Router::new().route(
		"/api/*fn_name",
		post(leptos_axum::handle_server_fns).get(leptos_axum::handle_server_fns),
	);

	let files = read_files(&config.leptos_options.site_root).await;

	for file in files {
		router = router.route_service(
			file.trim_start_matches(config.leptos_options.site_root.as_str()),
			ServeFile::new(file.as_str()),
		);
	}

	router
		.leptos_routes(
			&config.leptos_options,
			leptos_axum::generate_route_list(frontend::render),
			frontend::render,
		)
		.with_state(config.leptos_options)
		.with_state(state.clone())
}

fn read_files(path: &str) -> Pin<Box<dyn Future<Output = Vec<String>> + '_>> {
	Box::pin(async move {
		let mut files = Vec::new();
		let mut read_dir = fs::read_dir(path).await.expect("failed to read directory");
		while let Some(entry) = read_dir.next_entry().await.expect("failed to read entry") {
			let path = entry.path();
			if path.is_dir() {
				files.append(&mut read_files(path.to_str().unwrap()).await);
			} else {
				files.push(path.to_str().unwrap().to_string());
			}
		}
		files
	})
}