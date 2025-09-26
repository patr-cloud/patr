use crate::prelude::*;

/// The content to show when the user is not logged in
#[component]
pub fn LoggedOutContent() -> impl IntoView {
	view! {
		"Logged out content"
	}
}
