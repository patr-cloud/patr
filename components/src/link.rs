use crate::imports::*;

/// Link component to navigate to other pages
/// made with \<a\> tag instead of button tag
#[component]
pub fn Link(
	/// The Target of the Link
	// #[prop(into, optional)]
	// to: MaybeSignal<AppRoute>
	/// Click Handler, to be only used with the button variant,
	/// this NEEDS JavaScript to be enabled.
	#[prop(optional)]
	on_click: Option<Box<dyn FnMut(&ev::MouseEvent)>>,
	/// The Children of the Link, usually a \<p\> tag or simply
	/// the link text
	children: Children,
	/// Additional class names to apply to the link, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Color of the link
	#[prop(into, optional)]
	color: MaybeSignal<Color>,
	/// Button Variant i.e. a Submit button or a Link,
	/// Defaults to Button
	#[prop(into, optional)]
	variant: MaybeSignal<Variant>,
	/// Variant of the Link
	#[prop(into, optional)]
	styleVariant: MaybeSignal<LinkStyleVariant>,
) -> impl IntoView {
	// todo!("Maka a single link link componenet that switches based on the
	// variant.");
	let class = move || {
		format!(
			"fr-ct-ct {} {}",
			if styleVariant.get() == LinkStyleVariant::Contained {
				format!("btn btn-{}", color.get())
			} else {
				format!("btn-plain txt-{}", color.get())
			},
			class.get()
		)
	};

	view! {
			<div>
			{move || match variant.get() {
				Variant::Button => {
					view! {
						<button
							// on:click={move |e| {
							// 	if let Some(click) = &on_click {
							// 		e.prevent_default();
							// 		click(&e);
							// 	}
							// }}
							// class=class
						>
							// {children()}
						</button>
					}.into_any()
				},
				Variant::Link => {
					view! {<a
						// class=class
	>{
	// children()
	}</a>}.into_any()
				}
			}}
			</div>
			// <a class=class>{children()}</a>
		}
}
