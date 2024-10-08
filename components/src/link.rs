use web_sys::MouseEvent;

use crate::imports::*;

/// Link component to navigate to other pages
/// Use the variant prop to switch between <a/> and <button/>
/// tag
#[component]
pub fn Link(
	/// Specifies which type of button to use,
	/// "button" or "submit", to be only used with the button variant
	#[prop(into, optional, default = false.into())]
	should_submit: MaybeSignal<bool>,
	/// The Target of the Link, to be used with the link variant
	#[prop(into, optional)]
	to: MaybeSignal<String>,
	/// Click Handler, to be only used with the button variant,
	/// this NEEDS JavaScript to be enabled.
	#[prop(optional)]
	on_click: Option<ClickHandler>,
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
	r#type: MaybeSignal<Variant>,
	/// Variant of the Link
	#[prop(into, optional)]
	style_variant: MaybeSignal<LinkStyleVariant>,
	/// Whether the button is disabled or not
	#[prop(into, optional)]
	disabled: MaybeSignal<bool>,
) -> impl IntoView {
	let class = move || {
		format!(
			"flex items-center justify-center {} {}",
			class.get(),
			match style_variant.get() {
				LinkStyleVariant::Outlined => "btn-outline".to_string(),
				LinkStyleVariant::Contained => format!("btn btn-{}", color.get()),
				_ => format!("btn-plain text-{}", color.get()).to_string(),
			},
		)
	};

	let on_click = move |e: MouseEvent| {
		if let Some(click) = &on_click {
			e.prevent_default();
			click(&e);
		}
	};

	let to = store_value(to);
	let children = store_value(children);

	view! {
		{move || match r#type.get() {
			Variant::Link => {
				view! {
					<A href={move || to.with_value(|val| val.get())} class={class.clone()}>
						{children.with_value(|val| val())}
					</A>
				}
					.into_view()
			}
			Variant::Button => {
				view! {
					<button
						type={if should_submit.get() { "submit" } else { "button" }}
						on:click={on_click.clone()}
						disabled={move || disabled.get()}
						class={class.clone()}
					>
						{children.with_value(|val| val())}
					</button>
				}
					.into_view()
			}
		}}
	}
}
