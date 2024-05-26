use crate::prelude::*;

#[component]
pub fn Spinner(
	/// Additional class names to apply to the spinner, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	view! { <span class={move || format!("spinner {}", class.get())}></span> }
}
