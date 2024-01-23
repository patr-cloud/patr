use std::rc::Rc;

use web_sys::MouseEvent;

use crate::imports::*;

/// Link component to navigate to other pages
/// Use the variant prop to switch between \<a/> and \<button/>
/// tag
#[component]
pub fn Link(
	/// The Type of the button. This is directly passed to the
	/// type attribute of the button if variant is Button
	#[prop(into, optional, default = "button".into())]
	r#type: MaybeSignal<String>,
	/// The Target of the Link, to be used with the link variant
	#[prop(into, optional)]
	to: MaybeSignal<String>,
	/// Click Handler, to be only used with the button variant,
	/// this NEEDS JavaScript to be enabled.
	#[prop(optional)]
	on_click: Option<Rc<dyn Fn(&ev::MouseEvent)>>,
	/// The Children of the Link, usually a \<p\> tag or simply
	/// the link text
	children: ChildrenFn,
	/// Additional class names to apply to the link, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Color of the link
	#[prop(into, optional)]
	color: MaybeSignal<Color>,
	/// Button Variant i.e. a button or a Link,
	/// Defaults to Button
	#[prop(into, optional)]
	variant: MaybeSignal<Variant>,
	/// Variant of the Link
	#[prop(into, optional)]
	style_variant: MaybeSignal<LinkStyleVariant>,
) -> impl IntoView {
	let class = move || {
		format!(
			"fr-ct-ct {} {}",
			if style_variant.get() == LinkStyleVariant::Contained {
				format!("btn btn-{}", color.get())
			} else {
				format!("btn-plain txt-{}", color.get())
			},
			class.get()
		)
	};

	let cloned_type = move || r#type.get().clone();

	let on_click = move |e: MouseEvent| {
		if let Some(click) = &on_click {
			e.prevent_default();
			click(&e);
		}
	};

	view! {
		{move || match variant.get() {
			Variant::Link => {
				view! {
					<a
						href={to.clone()}
						class=class()
					>
						{children()}
					</a>
				}.into_view()
			},
			Variant::Button => {
				view! {
					<button
						type=cloned_type()
						on:click=on_click.clone()
						class=class()
					>
						{children()}
					</button>
				}.into_view()
			},
		}}
	}
}
