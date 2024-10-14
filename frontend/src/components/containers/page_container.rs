use crate::imports::*;

/// A Single Page container, typically used for LoggedOut set of Routes
#[component]
pub fn PageContainer(
	/// Additional classnames to appy to the outer div, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
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
			<main class="flex flex-col items-center justify-center w-full px-lg">{children()}</main>
		</div>
	}
}
