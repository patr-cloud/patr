use crate::imports::*;

/// Contains all the page titles, and wraps around indivisual <PageTitle />
/// components
#[component]
pub fn PageTitleContainer(
	/// Additional class names to apply to the outer div, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The Children of the component
	children: Children,
) -> impl IntoView {
	let class = move || format!("p-xxs fr-fs-ct {}", class.get());

	view! { <div class={class}>{children()}</div> }
}
