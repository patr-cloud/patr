#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]
//! Main dashboard console for Patr

/// Prelude module. Used to re-export commonly used items.
pub mod prelude {
	/// The global Portal ID for creating any portals
	pub struct PortalId;

	pub use leptos::*;
	pub use leptos_meta::{Link as MetaLink, Meta, Stylesheet, Title};
	pub use log::{debug, error, info, trace, warn};

	pub use crate::{components::*, pages::*, utils::*};
}

use leptos_meta::provide_meta_context;
use prelude::*;

/// The application logic code. This contains the routers and all the routing
/// logic
pub mod app;
/// The components module. This contains all the components used in the
/// application. Most reusable components like buttons, inputs, etc. are
/// all defined here.
pub mod components;
/// The pages module. This contains all the pages used in the application.
/// Pages are the main views that are rendered when a route is matched.
pub mod pages;
/// The utils module. This contains all the utility functions and other things
/// needed to make the application work.
pub mod utils;

/// The main hydrate function. Called when the application starts to hydrate
/// from the server side.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
	wasm_logger::init(wasm_logger::Config::default());

	if cfg!(debug_assertions) {
		console_error_panic_hook::set_once();
	}

	// Comment the below line to disable JS and test the app in pure SSR mode
	mount_to_body(render);
}

/// The main render function. Called when the application starts to render
/// from the client side.
pub fn render() -> View {
	use app::App;

	provide_meta_context();
	view! {
		<>
			<Meta charset="utf-8" />
			<MetaLink rel="shortcut icon" href="/favicon.svg" type_="image/svg+xml" />
			<MetaLink rel="apple-touch-icon" href="/favicon.svg" />
			<Meta name="viewport" content="width=device-width, initial-scale=1" />
			<Meta name="theme-color" content="#000000" />
			<Meta
				name="description"
				content="Patr: A code Deployment Platform that helps you scale what you build. You build, we scale"
			/>
			<MetaLink rel="preconnect" href="https://fonts.gstatic.com" />
			<MetaLink rel="preconnect" href="https://fonts.googleapis.com" />
			<MetaLink rel="preconnect" href="https://fonts.gstatic.com" crossorigin="" />
			<MetaLink
				href="https://fonts.googleapis.com/css2?family=PT+Serif:wght@700&family=Source+Code+Pro:wght@300;400&family=Poppins:wght@300;400;500;600;700&display=swap"
				rel="stylesheet"
			/>
			<MetaLink
				rel="stylesheet"
				href="https://cdnjs.cloudflare.com/ajax/libs/animate.css/4.1.1/animate.min.css"
			/>
			<Stylesheet id="leptos" href="/pkg/dashboard.css" />

			<Title formatter=|title: String| {
				if title.is_empty() {
					"Patr".to_string()
				} else {
					format!("{title} | Patr")
				}
			} />

			// <PortalProvider>
				<App/>
				// <PortalOutput id={PortalId}/>
			// </PortalProvider>
		</>
	}.into()
}
