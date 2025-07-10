//! Main dashboard console for Patr

#[cfg(target_arch = "wasm32")]
pub fn main() {
	frontend::start();
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
	use std::net::SocketAddr;

	use dioxus::{cli_config, prelude::*};

	axum::serve(
		tokio::net::TcpListener::bind(cli_config::fullstack_address_or_localhost())
			.await
			.unwrap(),
		axum::Router::new()
			.serve_dioxus_application(ServeConfigBuilder::new(), frontend::app::App)
			.into_make_service_with_connect_info::<SocketAddr>(),
	)
	.await
	.unwrap();
}
