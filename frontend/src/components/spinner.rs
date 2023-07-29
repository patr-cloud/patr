use crate::prelude::*;

#[component]
pub fn Spinner(
	/// The scope of the component.
	cx: Scope,
	/// Additional class names to apply to the spinner, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	view! { cx,
		<span class=move || format!("spinner {}", class.get()) />
	}
}
