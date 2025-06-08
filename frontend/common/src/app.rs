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

        if auth_state.read().is_logged_in() {
            "Dashboard"
        } else {
            LoginPage {}
        }
    }
}
