use crate::prelude::*;

#[component]
pub fn NotFound() -> impl IntoView {
	view! {
		<div class="fc-ct-ct bg-page bg-secondary">
			<h1 class="txt-primary txt-xl">404</h1>
			<h2 class="txt-grey txt-lg">Page Not Found</h2>
		</div>
	}
}
