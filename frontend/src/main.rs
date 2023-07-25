#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Main dashboard console for Patr

use prelude::*;

mod app;
mod components;
mod pages;
mod utils;

pub use app::prelude;
use app::App;
use wasm_bindgen::JsCast;

/// Main function. Called when the application is started.
/// Is only used when running the application directly.
/// If the application is used as a library, this function is not called.
pub fn main() {
	let root_element = document()
		.get_element_by_id("root")
		.expect("unable to find root element");
	mount_to(root_element.unchecked_into(), |cx| {
		view! {
			cx,
			<App />
		}
	});
}
