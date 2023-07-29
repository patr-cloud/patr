use wasm_bindgen::JsValue;

use crate::prelude::*;

/// An input field, with optional start and end icons, and an optional info
/// tooltip.
#[component]
pub fn Input(
	/// The scope of the component.
	cx: Scope,
	/// The ID of the input.
	#[prop(into, optional)]
	id: MaybeSignal<String>,
	/// Placeholder text for the input.
	#[prop(into, optional)]
	placeholder: MaybeSignal<String>,
	/// The type of input
	#[prop(into, default = "text".into())]
	type_: MaybeSignal<String>,
	/// Whether the input is disabled.
	#[prop(into, default = false.into())]
	disabled: MaybeSignal<bool>,
	/// The ref to forward to the input
	#[prop(into, optional)]
	ref_: Option<NodeRef<html::Input>>,
	/// The start icon, if any.
	#[prop(into, optional)]
	start_icon: MaybeSignal<Option<IconProps>>,
	/// The start text, if any.
	#[prop(into, optional)]
	start_text: MaybeSignal<Option<String>>,
	/// The end icon, if any.
	#[prop(into, optional)]
	end_icon: MaybeSignal<Option<IconProps>>,
	/// The end text, if any.
	#[prop(into, optional)]
	end_text: MaybeSignal<Option<String>>,
	/// The tooltip for the input.
	#[prop(into, optional)]
	info_tooltip: MaybeSignal<Option<String>>,
	/// Color variant of the input.
	#[prop(into, optional)]
	variant: MaybeSignal<SecondaryColorVariant>,
	/// Input event handler, if any
	#[prop(into, optional, default = Box::new(|_| ()))]
	on_input: Box<dyn FnMut(ev::Event)>,
	/// The initial value of the input.
	#[prop(into, optional)]
	value: MaybeSignal<String>,
	/// Additional class names to apply to the input, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let node_ref = ref_.unwrap_or_else(|| create_node_ref::<html::Input>(cx));

	let (input_value, set_input_value) = create_signal(cx, "test".to_string());

	view! { cx,
		<div class=move || format!(
			"input fr-fs-ct row-card bg-secondary-{} {}",
			variant.get().as_css_name(),
			class.get()
		)>
			{move || info_tooltip
				.get()
				.map(move |content| {
					TooltipContainer(
						cx,
						TooltipContainerProps {
							content: String::new(),
							label: None,
							disable_focus: false,
							icon_color: PatrColor::default(),
							variant: variant.get(),
							class: String::new(),
							children: Box::new(move |cx| content.clone().into_view(cx).into()),
						},
					)
				})}
			{
				start_icon
					.with(|props|
						props
							.as_ref()
							.map(|props|
								IconProps {
									icon: props.icon,
									size: props.size,
									color: props.color,
									class: props.class.clone(),
									click: props.click.clone(),
									enable_pulse: props.enable_pulse,
									fill: props.fill,
								}
							)
					)
					.into_view(cx)
			}
			{move || start_text.get()}
			{
				match value {
					MaybeSignal::Static(value) => {
						view! {cx,
							<input
								id={move || id.get()}
								ref={node_ref}
								class="mx-md of-hidden txt-of-ellipsis"
								disabled={move || disabled.get()}
								placeholder={move || placeholder.get()}
								on:input=on_input
								value=move || value.clone()
								type=move || type_.get() />
						}
					}
					MaybeSignal::Dynamic(value) => {
						view! {cx,
							<input
								id={move || id.get()}
								ref={node_ref}
								class="mx-md of-hidden txt-of-ellipsis"
								disabled={move || disabled.get()}
								placeholder={move || placeholder.get()}
								on:input=on_input
								prop:value={move || {
									JsValue::from_str(value.get().as_str())
								}}
								type=move || type_.get() />
						}
					}
				}
			}
			{move || end_text.get()}
			{
				end_icon
					.with(|props|
						props
							.as_ref()
							.map(|props|
								IconProps {
									icon: props.icon,
									size: props.size,
									color: props.color,
									class: props.class.clone(),
									click: props.click.clone(),
									enable_pulse: props.enable_pulse,
									fill: props.fill,
								}
							)
					)
					.into_view(cx)
			}
		</div>
	}
}
