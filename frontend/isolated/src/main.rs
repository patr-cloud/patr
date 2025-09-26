//! A web dashboard for the application. This is isolated from the main API
//! server to allow for easier rebuilds

use std::net::SocketAddr;

use axum::{
	Router,
	body::{Body, Bytes},
	http::{HeaderMap, Response, Uri},
	routing::any,
};
use frontend::utils::AppType;
use leptos::prelude::*;
use leptos_axum::LeptosRoutes;
use reqwest::Method;
use tokio::{fs, net::TcpListener};
use tower_http::services::ServeFile;

/// Sets up the routes for the web dashboard
#[tokio::main]
pub async fn main() {
	let config = get_configuration(
		if option_env!("LEPTOS_OUTPUT_NAME").is_some() {
			None
		} else {
			Some(concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.toml"))
		},
	)
	.expect("failed to get configuration");

	let router = read_files(&config.leptos_options.site_root)
		.await
		.into_iter()
		.fold(Router::new(), |router, file| {
			router.route_service(
				file.trim_start_matches(config.leptos_options.site_root.as_ref()),
				ServeFile::new(file.as_str()),
			)
		})
		.leptos_routes(
			&config.leptos_options,
			{
				let leptos_options = config.leptos_options.clone();
				leptos_axum::generate_route_list(move || {
					frontend::render(leptos_options.clone(), AppType::Managed)
				})
			},
			{
				let leptos_options = config.leptos_options.clone();
				move || frontend::render(leptos_options.clone(), AppType::Managed)
			},
		)
		.route(
			"/{*any}",
			// Proxy the request as it is, with path, query params,
			// headers and body to localhost:3000
			any(
				|method: Method, uri: Uri, headers: HeaderMap, body: Bytes| async move {
					let client = reqwest::Client::new();
					let target_url = format!(
						"http://localhost:3000{}",
						uri.path_and_query()
							.map(|pq| pq.as_str())
							.unwrap_or(uri.path())
					);

					let response = client
						.request(method, &target_url)
						.headers(headers)
						.body(body)
						.send()
						.await
						.inspect_err(|err| {
							eprintln!("Error proxying request to backend: {err}");
						});

					match response {
						Ok(resp) => {
							let status = resp.status();
							let headers = resp.headers().clone();
							match resp.bytes().await {
								Ok(body) => {
									let mut response = Response::new(Body::from(body));
									*response.status_mut() = status;
									*response.headers_mut() = headers;
									response
								}
								Err(_) => Response::builder()
									.status(502)
									.body(Body::from("Failed to read response body"))
									.unwrap(),
							}
						}
						Err(_) => Response::builder()
							.status(502)
							.body(Body::from("Bad Gateway"))
							.unwrap(),
					}
				},
			),
		)
		.with_state(config.leptos_options);

	axum::serve(
		TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 4001)))
			.await
			.unwrap(),
		router.into_make_service_with_connect_info::<SocketAddr>(),
	)
	.await
	.unwrap();
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
