use crate::prelude::*;

/// Icon component. Used to display icons from the Feather icon set.
#[component]
pub fn Icon(
	/// scope of the component
	cx: Scope,
	/// name of the icon to display
	#[prop(into)]
	icon: String,
	/// class name to apply to the icon
	#[prop(optional, into)]
	class_name: String,
	/// text color of the icon
	#[prop(optional)]
	color: PatrColor,
	/// fill color of the icon
	#[prop(optional)]
	fill: PatrColor,
	/// size of the icon
	#[prop(optional)]
	size: Size,
	/// whether to enable the pulse animation
	#[prop(optional)]
	enable_pulse: bool,
	/// click handler
	#[prop(optional)]
	click: Option<Box<dyn Fn(leptos::ev::MouseEvent)>>,
) -> impl IntoView {
	let is_clickable = click.is_some();

	view! {
		cx,
		<svg
			class="icon"
			class=class_name
			class:pulse=enable_pulse
			class=color.as_css_text_color()
			class=format!("icon-fill-{}", fill.as_css_name())
			class=format!("icon-{}", size.as_css_name())
			class=("cursor-pointer", is_clickable)
			on:click=if let Some(click) = click {
				click
			} else {
				Box::new(|_| ())
			}
		>
			<use_ href=format!("{}#{icon}", constants::FEATHER_IMG) />
		</svg>
	}
}
