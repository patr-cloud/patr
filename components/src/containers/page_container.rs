use crate::imports::*;

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
			"fr-fs-fs bg-page-container full-width full-height bg-secondary {}",
			class.get()
		)
	};

	view! {
		<div class=class>
			<main class="fc-ct-ct full-width px-lg">
				{children()}
			</main>
		</div>
	}
}
