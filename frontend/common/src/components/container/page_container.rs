use crate::prelude::*;

/// A Single Page container, typically will wrap around all pages
#[component]
pub fn PageContainer(
	/// Additional Class Names to apply to the outer div, if any
	#[prop(into, optional)]
	class: Signal<String>,
	/// The contents of the page
	children: Children,
) -> impl IntoView {
	let class = move || {
		format!(
			"flex items-start justify-start bg-page-container w-full h-full bg-secondary {}",
			class.get()
		)
	};

	view! {
		<div class={class}>
			<main class="flex flex-col items-center justify-center w-full px-lg h-full">{children()}</main>
		</div>
	}
}
