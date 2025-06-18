//! Main dashboard console for Patr

#[cfg(target_arch = "wasm32")]
fn main() {
	use log::Level;
	use wasm_logger::Config;

	console_error_panic_hook::set_once();
	wasm_logger::init(
		Config::new(Level::Trace)
			.message_on_new_line()
			.module_prefix("common"),
	);

	dioxus::LaunchBuilder::web().launch(frontend::app::App);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
	use dioxus::{cli_config, prelude::*};

	axum::serve(
		tokio::net::TcpListener::bind(cli_config::fullstack_address_or_localhost())
			.await
			.unwrap(),
		axum::Router::new()
			.serve_dioxus_application(ServeConfigBuilder::new(), frontend::app::App)
			.into_make_service(),
	)
	.await
	.unwrap();
}
