use crate::imports::*;

#[component]
pub fn Backdrop(
	/// The Children of the Backdrop
	children: ChildrenFn,
	/// Color variant of the backdrop
	variant: SecondaryColorVariant,
	/// Additional classnames to apply to the outer div, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || {
		class.with(|classname| {
			format!(
				"bg-page bg-backdrop flex items-center justify-center backdrop-{} {}",
				variant.as_css_name(),
				classname,
			)
		})
	};
	view! { <div class={class}>{children()}</div> }
}
