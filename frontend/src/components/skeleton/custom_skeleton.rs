use crate::imports::*;

#[component]
pub fn CustomSkeleton(
	/// Additional class names to apply to the outer table, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || class.with(|classname| format!("custom-skeleton {}", classname));

	view! { <div class={class}></div> }
}
