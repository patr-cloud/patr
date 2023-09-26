#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! Main dashboard console for Patr

/// Prelude module. Used to re-export commonly used items.
pub mod prelude {
	/// The global Portal ID for creating any portals
	pub struct PortalId;

	pub use leptos::*;
	pub use log::{debug, error, info, trace, warn};

	pub use crate::{components::*, pages::*, utils::*};
}

use leptos_declarative::prelude::*;
use prelude::*;
use leptos_meta::{Meta, Link, Title, Stylesheet, provide_meta_context};

pub mod app;
pub mod components;
pub mod pages;
pub mod utils;

use app::App;

/// The main hydrate function. Called when the application starts to hydrate
/// from the server side.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
	wasm_logger::init(wasm_logger::Config::default());

	if cfg!(debug_assertions) {
		console_error_panic_hook::set_once();
	}

	mount_to_body(render);
}

/// The main render function. Called when the application starts to render
/// from the client side.
pub fn render(cx: Scope) -> View {
	provide_meta_context(cx);
	view! { cx,
		<>
			<Meta charset="utf-8" />
			<Link rel="shortcut icon" href="/favicon.svg" type_="image/svg+xml" />
			<Link rel="apple-touch-icon" href="/favicon.svg" />
			<Meta name="viewport" content="width=device-width, initial-scale=1" />
			<Meta name="theme-color" content="#000000" />
			<Meta
				name="description"
				content="Patr: A code Deployment Platform that helps you scale what you build. You build, we scale"
			/>
			<Link rel="preconnect" href="https://fonts.gstatic.com" />
			<Link rel="preconnect" href="https://fonts.googleapis.com" />
			<Link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="" />
			<Link
				href="https://fonts.googleapis.com/css2?family=PT+Serif:wght@700&family=Source+Code+Pro:wght@300;400&family=Poppins:wght@300;400;500;600;700&display=swap"
				rel="stylesheet"
			/>
			<Link
				rel="stylesheet"
				href="https://cdnjs.cloudflare.com/ajax/libs/animate.css/4.1.1/animate.min.css"
			/>
			<Stylesheet id="leptos" href="/dashboard.css" />

			<Title formatter=|title: String| {
				if title.is_empty() {
					"Patr".to_string()
				} else {
					format!("{title} | Patr")
				}
			} />

			<PortalProvider>
				<App/>
				<PortalOutput id={PortalId}/>
			</PortalProvider>
		</>
	}.into()
}
