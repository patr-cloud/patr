#![feature(impl_trait_in_assoc_type)]

//! Main dashboard console for Patr

/// Prelude module. Used to re-export commonly used items.
pub mod prelude {
	pub use leptos::prelude::*;
	pub use leptos_router::*;
	pub use leptos_use::use_cookie;
	// pub use models::prelude::*;
	pub use models::prelude::Uuid;

	pub use crate::{components::*, pages::*, utils::*};
}

/// The API Module. This contains all the server functions that are used
/// to make API calls to the backend.
// pub mod api;
/// The application logic code. This contains the routers and all the routing
/// logic
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

use leptos_meta::{Title, provide_meta_context};
use prelude::*;

#[cfg(target_arch = "wasm32")]
/// The main hydrate function. Called when the application starts to hydrate
/// from the server side.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
	wasm_logger::init(wasm_logger::Config::default());

	if cfg!(debug_assertions) {
		console_error_panic_hook::set_once();
	}

	// Comment the below line to disable JS and test the app in pure SSR mode
	leptos::mount::hydrate_body(app::App);
}

/// The main render function. Called when the application starts to render
/// from the client side.
pub fn render(options: LeptosOptions) -> impl IntoView {
	use app::App;

	provide_meta_context();

	view! {
		<!DOCTYPE html>
		<html lang="en">
			<head>
				<meta charset="utf-8" />
				<link rel="shortcut icon" href="/favicon.svg" type_="image/svg+xml" />
				<link rel="apple-touch-icon" href="/favicon.svg" />
				<meta name="viewport" content="width=device-width, initial-scale=1" />
				<meta name="theme-color" content="#000000" />
				<meta
					name="description"
					content="Patr: A code Deployment Platform that helps you scale what you build. You build, we scale"
				/>
				<link rel="preconnect" href="https://fonts.gstatic.com" />
				<link rel="preconnect" href="https://fonts.googleapis.com" />
				<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="" />
				<link
					href="https://fonts.googleapis.com/css2?family=PT+Serif:wght@700&family=Source+Code+Pro:wght@300;400&family=Poppins:wght@300;400;500;600;700&display=swap"
					rel="stylesheet"
				/>
				<link
					rel="stylesheet"
					href="https://cdnjs.cloudflare.com/ajax/libs/animate.css/4.1.1/animate.min.css"
				/>
				<link rel="stylesheet" href="/pkg/dashboard.css" />
				<AutoReload options=options.clone() />
				<HydrationScripts options/>

				<Title
					text="Patr"
				/>
			</head>

			<body>
				<App />
			</body>
		</html>
	}
}
