use leptos_router::{use_navigate, NavigateOptions};

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
	/// The target of the link.
	#[prop(into, optional)]
	to: MaybeSignal<AppRoute>,
	/// The color of the link
	#[prop(into, optional)]
	color: MaybeSignal<Color>,
	/// The variant of the link
	#[prop(into, optional)]
	variant: MaybeSignal<LinkVariant>,
	/// Whether the link is disabled
	#[prop(into, optional)]
	disabled: MaybeSignal<bool>,
	/// The type of the button. This is directly passed to the `type` attribute
	/// of the button.
	#[prop(into, optional, default = "button".into())]
	r#type: MaybeSignal<String>,
	/// Additional class names to apply to the link, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// click handler
	#[prop(optional)]
	on_click: Option<Box<dyn Fn(&ev::MouseEvent)>>,
	/// The children of the link, if any
	children: Children,
) -> impl IntoView {
	let navigate = use_navigate();

	view! {
		<button
			type=move || r#type.get()
			on:click={move |e| {
				let mut navigate_page = true;
				if let Some(click) = &on_click {
					click(&e);
					navigate_page = !e.default_prevented();
				}
				if navigate_page {
					if !to.get().is_empty() {
						navigate(to.get().to_string().as_str(), NavigateOptions::default());
					}
				}
			}}
			disabled=move || disabled.get()
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
			{children()}
		</button>
	}
}