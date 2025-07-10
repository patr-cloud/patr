//! Main dashboard console for Patr

/// Prelude module. Used to re-export commonly used items.
pub(crate) mod prelude {
	pub use dioxus::prelude::*;
	pub use models::prelude::*;

	pub use crate::{components::*, pages::*, utils::*};
}

/// The main app that gets launched when the application starts.
pub mod app;
/// The components module. This module contains all the components that are used
/// across all applications in the Patr ecosystem.
pub mod components;
/// The pages module. This contains all the pages used in the application.
/// Pages are the main views that are rendered when a route is matched.
pub mod pages;
/// The utils module. This contains all the utility functions and other things
/// needed to make the application work.
pub mod utils;

#[cfg(target_arch = "wasm32")]
pub fn start() {
	use log::Level;
	use wasm_logger::Config;

	console_error_panic_hook::set_once();
	wasm_logger::init(
		Config::new(Level::Trace)
			.message_on_new_line()
			.module_prefix("common"),
	);

	dioxus::LaunchBuilder::web().launch(app::App);
}
