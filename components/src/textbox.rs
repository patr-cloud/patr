use crate::imports::*;

#[component]
pub fn Textbox(
	/// The Start Icon if any
	#[prop(into, optional)]
	start_icon: MaybeSignal<Option<IconProps>>,
	/// The End Icon if any
	#[prop(into, optional)]
	end_icon: MaybeSignal<Option<IconProps>>,
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	#[prop(optional)]
	/// The Color Variant of the textbox
	color_variant: SecondaryColorVariant,
	/// Whether the textbox is disable.
	#[prop(into, optional)]
	disabled: MaybeSignal<bool>,
	/// A Default Value, passed to the span,
	/// Takes first priority
	#[prop(into, optional, default = None)]
	value: Option<View>,
	/// A placeholder for the "input", takes 3rd priority
	/// after value,
	#[prop(into, optional)]
	placeholder: MaybeSignal<String>,
	// on_click: Option<Rc<dyn Fn(&ev::MouseEvent)>>,
	/// The length, use 0 to not use any ellipsis, defaults to 0
	#[prop(optional, default = 0)]
	ellipsis: i32,
) -> impl IntoView {
	let class = class.with(|cname| {
		format!(
			"flex justify-start items-center py-sm px-xl br-sm text-medium row-card w-full bg-secondary-{} {}",
			color_variant.as_css_name(),
			cname
		)
	});

	let span_class = format!(
		"px-md mr-auto text-ellipsis {} {}",
		match ellipsis {
			0 => "".to_owned(),
			n => format!("text-ellipsis overflow-hidden w-{n}"),
		},
		if value.is_none() || disabled.get() {
			"txt-disabled"
		} else {
			"txt-white"
		}
	);
	view! {
		<div class={class}>

			{start_icon
				.with(|props| {
					props
						.as_ref()
						.map(|props| IconProps {
							icon: props.icon,
							size: props.size,
							color: props.color,
							class: props.class.clone(),
							on_click: props.on_click.clone(),
							enable_pulse: props.enable_pulse,
							fill: props.fill,
						})
				})}
			<span class={span_class}>

				{if value.is_some() { value.into_view() } else { placeholder.into_view() }}

			</span>
			{end_icon
				.with(|props| {
					props
						.as_ref()
						.map(|props| IconProps {
							icon: props.icon,
							size: props.size,
							color: props.color,
							class: props.class.clone(),
							on_click: props.on_click.clone(),
							enable_pulse: props.enable_pulse,
							fill: props.fill,
						})
				})}

		</div>
	}
}
