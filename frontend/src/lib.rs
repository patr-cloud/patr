#![feature(impl_trait_in_assoc_type)]

//! Main dashboard console for Patr

/// Prelude module. Used to re-export commonly used items.
pub mod prelude {
	pub use leptos::*;
	pub use leptos_router::*;
	pub use leptos_use::use_cookie;
	pub use models::prelude::*;

	pub use crate::{
		api::*,
		components::{
			alert::*,
			backdrop::*,
			checkbox_dropdown::*,
			containers::*,
			dashboard_container::*,
			double_input_slider::*,
			error_page::*,
			icon::*,
			input::*,
			input_dropdown::*,
			link::*,
			modal::*,
			number_picker::*,
			otp_input::*,
			page_title::*,
			popover::*,
			sidebar::*,
			skeleton::*,
			spinner::*,
			status_badge::*,
			table_dashboard::*,
			textbox::*,
			toast::*,
			tooltip::*,
		},
		routes::*,
		utils::*,
	};
}

/// The imports module. This is basically similar to a prelude, but for within
/// the crate
mod imports {
	use std::rc::Rc;

	/// The handler for the click event on a component. This can be either a
	/// function or a closure that takes a MouseEvent as an argument.
	pub(crate) type ClickHandler = Rc<dyn Fn(&ev::MouseEvent)>;

	pub use leptos::*;
	pub use leptos_router::A;

	pub use crate::prelude::*;
}

/// The API Module. This contains all the server functions that are used
/// to make API calls to the backend.
pub mod api;
/// The application logic code. This contains the routers and all the routing
/// logic
pub mod app;
/// The components module. This module contains all the components that are used
/// across all applications in the Patr ecosystem.
pub mod components;
/// The pages module. This contains all the pages used in the application.
/// Pages are the main views that are rendered when a route is matched.
pub mod pages;
/// All the Leptos Queries and Tags used in the API routes
pub mod queries;
/// The Routes module. This contains all the routes used in the applica.
/// Routes are what defines the URL for each and every page.
pub mod routes;
/// The utils module. This contains all the utility functions and other things
/// needed to make the application work.
pub mod utils;

use leptos_meta::{provide_meta_context, Link as MetaLink, Meta, Stylesheet, Title};
use prelude::*;

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
pub fn render() -> impl IntoView {
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

			<Title formatter={|title: String| {
				if title.is_empty() { "Patr".to_string() } else { format!("{title} | Patr") }
			}} />

			<App />
		</>
	}
}
