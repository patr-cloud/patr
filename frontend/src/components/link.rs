use leptos_router::use_navigate;

use crate::prelude::*;

/// The type of link to use. A contained link is a button with a background,
/// while a plain link looks like an anchor tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LinkVariant {
	/// A contained link. This is a button with a background.
	Contained,
	/// A plain link. This looks like an anchor tag.
	#[default]
	Plain,
}

/// Link component to navigate to other pages
#[component]
pub fn Link(
	/// The scope of the component
	cx: Scope,
	/// The target of the link. TODO make this an enum
	#[prop(into, optional)]
	to: MaybeSignal<String>,
	/// The color of the link
	#[prop(into, optional)]
	color: MaybeSignal<PatrColor>,
	/// The variant of the link
	#[prop(into, optional)]
	variant: MaybeSignal<LinkVariant>,
	/// Whether the link is disabled
	#[prop(into, optional)]
	disabled: MaybeSignal<bool>,
	/// The type of the button. This is directly passed to the `type` attribute
	/// of the button.
	#[prop(into, optional, default = "button".into())]
	type_: MaybeSignal<String>,
	/// Additional class names to apply to the link, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// click handler
	#[prop(into, optional)]
	click: Option<Box<dyn Fn(ev::MouseEvent)>>,
	/// The children of the link, if any
	children: Children,
) -> impl IntoView {
	// let navigate = use_navigate(cx);

	view! { cx,
		<button
			type_={move || type_.get()}
			on:click={move |e| {
				if !to.get().is_empty() {
					// _ = navigate(to.get().as_str(), Default::default());
				}
				if let Some(click) = &click {
					click(e);
				}
			}}
			disabled={move || disabled.get()}
			class={move || format!(
				"fr-ct-ct {} {}",
				if variant.get() == LinkVariant::Contained {
					format!("btn btn-{}", color.get())
				} else {
					format!("btn-plain txt-{}", color.get())
				},
				class.get()
			)}
		>
			{children(cx)}
		</button>
	}
}
