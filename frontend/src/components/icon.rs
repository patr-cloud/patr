use crate::prelude::*;

pub struct Def;

impl Fn<ev::MouseEvent> for Def {
	fn call(&self, _: ev::MouseEvent) {}
}

/// Icon component. Used to display icons from the Feather icon set.
#[component]
pub fn Icon(
	/// scope of the component
	cx: Scope,
	/// name of the icon to display
	#[prop(into)]
	icon: MaybeSignal<String>,
	/// class name to apply to the icon
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// text color of the icon
	#[prop(into, optional, default = White.into())]
	color: MaybeSignal<PatrColor>,
	/// fill color of the icon
	#[prop(into, optional)]
	fill: MaybeSignal<PatrColor>,
	/// size of the icon
	#[prop(into, optional)]
	size: MaybeSignal<Size>,
	/// Whether to enable the pulse animation
	#[prop(into, optional)]
	enable_pulse: MaybeSignal<bool>,
	/// click handler
	#[prop(into, optional)]
	click: Option<Box<dyn Fn(ev::MouseEvent)>>,
) -> impl IntoView {
	let is_clicked = click.is_some();

	view! { cx,
		<svg
			class=move || format!(
				"icon {} {} icon-fill-{} icon-{} {} {}",
				if enable_pulse.get() {
					"pulse"
				} else {
					""
				},
				color.get().as_text_color().as_css_color(),
				fill.get().as_css_name(),
				size.get().as_css_name(),
				if is_clicked {
					"cursor-pointer"
				} else {
					""
				},
				class.get()
			)
			on:click=if let Some(click) = click {
				click
			} else {
				Box::new(|_| ())
			}
		>
			<use_ href={move || format!("{}#{}", constants::FEATHER_IMG, icon.get())} />
		</svg>
	}
}
