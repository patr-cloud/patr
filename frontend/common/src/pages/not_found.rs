use crate::prelude::*;

/// The Not Found page. This page is displayed when the user tries to access a
/// route that does not exist or is not defined in the application.
#[component]
pub fn NotFoundPage() -> Element {
	rsx! {
		PageContainer { class: "bg-onboard",
			h1 { class: "text-primary", "404 - Not Found" }
		}
	}
}
