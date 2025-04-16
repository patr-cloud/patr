//! This crate is a wrapper around the [`frontend`] crate, which is a
//! Leptos application, specifically used to set the
//! [`AppType`][frontend::prelude::AppType] to the self-hosted mode.

#[cfg(target_arch = "wasm32")]
/// The main hydrate function. Called when the application starts to hydrate
/// from the server side.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
	use frontend::{
		app::{App, AppProps},
		utils::AppType,
	};

	wasm_logger::init(wasm_logger::Config::default());

	if cfg!(debug_assertions) {
		console_error_panic_hook::set_once();
	}

	// Comment the below line to disable JS and test the app in pure SSR mode
	leptos::mount::hydrate_body(|| App(AppProps::builder().app_type(AppType::SelfHosted).build()));
}

#[cfg(all(not(debug_assertions), not(target_arch = "wasm32")))]
compile_error!(concat!(
	"This crate is only intended to be used as a WASM target. ",
	"To test this crate, use the `--target=wasm32-unknown-unknown` flag. ",
	"If you are not manually compiling this, you are doing something wrong."
));
