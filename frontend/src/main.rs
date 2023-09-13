#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! Main dashboard console for Patr

/// Prelude module. Used to re-export commonly used items.
pub mod prelude {
	/// The global Portal ID for creating any portals
	pub struct PortalId;

	pub use leptos::*;

	pub use crate::{components::*, pages::*, utils::*};

	pub use models::prelude::*;
}

use leptos_declarative::prelude::*;
use prelude::*;

mod app;
mod components;
mod pages;
mod utils;

use app::App;
use wasm_bindgen::JsCast;

/// Main function. Called when the application is started.
/// Is only used when running the application directly.
/// If the application is used as a library, this function is not called.
pub fn main() {
	wasm_logger::init(wasm_logger::Config::default());

	if cfg!(debug_assertions) {
		console_error_panic_hook::set_once();
	}

	let root_element = document()
		.get_element_by_id("root")
		.expect("unable to find root element");
	mount_to(root_element.unchecked_into(), |cx| {
		view! { cx,
			<PortalProvider>
				<App/>
				<PortalOutput id={PortalId}/>
			</PortalProvider>
		}
	});
}
