use crate::{pages::*, prelude::*};

#[component]
fn LoggedInPage() -> impl IntoView {
	view! {
		<div class="fr-fs-fs full-width full-height bg-secondary">
			<Sidebar />
			<main class="fc-fs-ct full-width px-lg">
				// This is a temporary empty div for the header
				<header style="width: 100%; min-height: 5rem;">
				</header>

				<ManageProfile />
			</main>
		</div>
	}
}

#[component]
pub fn App() -> impl IntoView {
	view! {
		<LoggedInPage />
	}
}
