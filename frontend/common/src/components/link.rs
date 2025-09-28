use leptos_router::components::A;

use crate::prelude::*;

/// Link component to navigate to other pages, wraps around HTML a tag, with
/// additional props for styling and such
#[component]
pub fn Link(
	/// The Children of the Link, usually a \<p\> tag or simply
	/// the link text
	children: ChildrenFn,
	/// Additional class names to apply to the link, if any
	#[prop(into, optional)]
	class: Signal<String>,
	/// Variant of the Link
	#[prop(into, optional)]
	variant: Signal<LinkStyleVariant>,
	/// The Target of the Link, to be used with the link variant
	#[prop(into, optional)]
	to: Signal<String>,
	/// Color of the link
	#[prop(into, optional)]
	color: Signal<Color>,
	/// The Target of the Link
	#[prop(optional)]
	target: LinkTarget,
) -> impl IntoView {
	let class = move || {
		format!(
			"flex items-center justify-center {} {}",
			class.get(),
			match variant.get() {
				LinkStyleVariant::Outlined => "btn-outline".to_string(),
				LinkStyleVariant::Contained => format!("btn btn-{}", color.get()),
				_ => format!("btn-plain text-{}", color.get()).to_string(),
			},
		)
	};

	view! {
		<A target={target.to_string()} attr:class={class} href={move || to.get()}>{children()}</A>
	}
}
