use crate::prelude::*;

#[component]
pub fn PageContainer(
	/// The children of the component
	children: Children,
) -> impl IntoView {
	// TODO check if workspace exists, and if it doesn't, show a create
	// workspace page
	view! {
		<div class="fr-fs-fs full-width full-height bg-secondary">
		 	<Sidebar />
			<main class="fc-fs-ct full-width px-lg">
				<TopNav />
				{children()}
			</main>
		</div>
	}
}
