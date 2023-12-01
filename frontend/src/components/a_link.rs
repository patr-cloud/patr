use crate::prelude::*;

/// Link component to navigate to other pages
/// made with \<a\> tag instead of button tag
#[component]
pub fn ALink(
	/// The target of the link
	#[prop(into, optional)]
	to: MaybeSignal<AppRoute>,
	/// The Children of the link if any
	children: Children,
	/// Additional class names to appy to the link, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Variant of the link
	#[prop(into, optional)]
	variant: MaybeSignal<LinkVariant>,
	/// Color of the link
	#[prop(into, optional)]
	color: MaybeSignal<Color>,
) -> impl IntoView {
	let class = move || {
		format!(
			"fr-ct-ct {} {}",
			if variant.get() == LinkVariant::Contained {
				format!("btn btn-{}", color.get())
			} else {
				format!("btn-plain txt-{}", color.get())
			},
			class.get()
		)
	};

	view! {
		<a href={move || to.get().to_string() }
			class=class
		>
			{children()}
		</a>
	}
}
