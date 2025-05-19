use crate::prelude::*;

#[component]
pub fn Spinner(
	/// Additional classes to apply to the spinner, if any
	#[props(into, optional)]
	class: String,
) -> Element {
	rsx! {
		span { class: "spinner {class}" }
	}
}
