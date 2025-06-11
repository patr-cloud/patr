use crate::prelude::*;

/// The main route for the application. This is the top-level route that
/// contains all the other routes. It is responsible for rendering the main
/// layout and the main content of the application. It is also responsible for
/// handling the routing and the state of the application.
#[derive(Debug, Clone, PartialEq, Eq, Routable)]
pub enum AppRoute {
	/// The route for the logged-in pages. This is the main route that contains
	/// all the pages that require the user to be logged in.
	#[child("/")]
	LoggedIn {
		/// The child route for the logged-in pages. This is the main route that
		/// contains all the pages that require the user to be logged in.
		child: LoggedInRoutes,
	},
	/// The route for the logged-out pages. This is the main route that contains
	/// all the pages that do not require the user to be logged in.
	#[child("/")]
	LoggedOut {
		/// The child route for the logged-out pages. This is the main route
		/// that contains all the pages that do not require the user to be
		/// logged in.
		child: LoggedOutRoutes,
	},
	/// The route for the Not Found page. This is the page that is displayed
	/// when the user tries to access a route that does not exist.
	#[route("/not-found", NotFoundPage)]
	NotFound,
}

/// The main app that gets launched when the application starts.
/// This is the main entry point for the application. It is responsible for
/// rendering the main layout and the main content of the application.
/// It is also responsible for handling the routing and the state of the
/// application.
#[component]
pub fn App() -> Element {
	let auth_state = AuthState::load();
	use_context_provider(|| auth_state);

	rsx! {
		document::Meta { charset: "utf-8" }
		document::Link {
			href: constants::FAVICON,
			r#type: "image/svg+xml",
			rel: "shortcut icon",
		}
		document::Link { href: constants::FAVICON, rel: "apple-touch-icon" }
		document::Meta { content: "width=device-width, initial-scale=1", name: "viewport" }
		document::Meta { content: "#000000", name: "theme-color" }
		document::Meta {
			name: "description",
			content: "Patr: A code Deployment Platform that helps you scale what you build. You build, we scale",
		}
		document::Link { href: "https://fonts.gstatic.com", rel: "preconnect" }
		document::Link { href: "https://fonts.googleapis.com", rel: "preconnect" }
		document::Link {
			crossorigin: "",
			rel: "preconnect",
			href: "https://fonts.gstatic.com",
		}
		document::Link {
			href: "https://fonts.googleapis.com/css2?family=PT+Serif:wght@700&family=Source+Code+Pro:wght@300;400&family=Poppins:wght@300;400;500;600;700&display=swap",
			rel: "stylesheet",
		}
		document::Link {
			rel: "stylesheet",
			href: "https://cdnjs.cloudflare.com/ajax/libs/animate.css/4.1.1/animate.min.css",
		}
		document::Link { href: constants::GLOBAL_CSS, rel: "stylesheet" }
		document::Title { "Patr" }

		Router::<AppRoute> { config: || RouterConfig::default().on_update(|_| on_router_update()) }
	}
}

fn on_router_update() -> Option<NavigationTarget<AppRoute>> {
	// TODO refresh auth state here
	None
}
