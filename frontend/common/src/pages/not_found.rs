use crate::prelude::*;

/// The 404 Not Found Page. This is usually shown for unknown routes
#[component]
pub fn NotFoundPage() -> impl IntoView {
	view! {
		<div class="not-found-page">
			<h1>"404 - Page Not Found"</h1>
			<p>"The page you are looking for does not exist."</p>
		</div>
	}
}
