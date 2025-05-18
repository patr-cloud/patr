use crate::{pages::*, prelude::*};

/// The main app that gets launched when the application starts.
/// This is the main entry point for the application. It is responsible for
/// rendering the main layout and the main content of the application.
/// It is also responsible for handling the routing and the state of the
/// application.
#[component]
pub fn App() -> Element {
	let auth_state = AuthState::load();

	rsx! {
        head {
            meta { charset: "utf-8" }
            link {
                href: constants::FAVICON,
                "type_": "image/svg+xml",
                rel: "shortcut icon",
            }
            link { href: constants::FAVICON, rel: "apple-touch-icon" }
            meta {
                content: "width=device-width, initial-scale=1",
                name: "viewport",
            }
            meta { content: "#000000", name: "theme-color" }
            meta {
                name: "description",
                content: "Patr: A code Deployment Platform that helps you scale what you build. You build, we scale",
            }
            link { href: "https://fonts.gstatic.com", rel: "preconnect" }
            link { href: "https://fonts.googleapis.com", rel: "preconnect" }
            link {
                crossorigin: "",
                rel: "preconnect",
                href: "https://fonts.gstatic.com",
            }
            link {
                href: "https://fonts.googleapis.com/css2?family=PT+Serif:wght@700&family=Source+Code+Pro:wght@300;400&family=Poppins:wght@300;400;500;600;700&display=swap",
                rel: "stylesheet",
            }
            link {
                rel: "stylesheet",
                href: "https://cdnjs.cloudflare.com/ajax/libs/animate.css/4.1.1/animate.min.css",
            }
            link { href: constants::GLOBAL_CSS, rel: "stylesheet" }
            title { "Patr" }
        }
        body {
            if auth_state.read().is_logged_in() {
                "Dashboard"
            } else {
                LoginPage {}
            }
        }
    }
}
