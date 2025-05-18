//! Main dashboard console for Patr

use log::Level;
use wasm_logger::Config;

fn main() {
	console_error_panic_hook::set_once();
	wasm_logger::init(Config::new(Level::Trace).message_on_new_line());
	tracing_wasm::set_as_global_default();

	dioxus::LaunchBuilder::web().launch(frontend::app::App);
}
