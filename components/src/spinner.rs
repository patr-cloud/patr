use crate::imports::*;

#[component]
pub fn Spinner(
	/// Additional classes to apply to the spinner, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || format!("spinner {}", class.get());
	view! { <span class={class}></span> }
}
