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
	let class = move || {
		class.with(|classname| {
			format!(
				"skeleton {classname} {} {}",
				if enable_full_height { "h-full" } else { "" },
				if enable_full_width { "w-full" } else { "" }
			)
		})
	};

	view! { <div class={class}></div> }
}
