use components::{pages::*, sidebar::Sidebar_TEST_Page};
use leptos::*;

fn main() {
	wasm_logger::init(wasm_logger::Config::default());

	if cfg!(debug_assertions) {
		console_error_panic_hook::set_once();
	}

	mount_to_body(Sidebar_TEST_Page)
}
