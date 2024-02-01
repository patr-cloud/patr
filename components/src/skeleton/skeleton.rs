use crate::imports::*;

#[component]
pub fn Skeleton(
	/// Additional class names to apply to the outer div, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Enable Full Width
	#[prop(optional, default = false)]
	enable_full_width: bool,
	/// Enable Full Height
	#[prop(optional, default = false)]
	enable_full_height: bool,
) -> impl IntoView {
	let class = move || format!("skeleton {}", class.get());

	view! {
		<div class=class></div>
	}
}
