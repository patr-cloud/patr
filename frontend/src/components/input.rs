use std::rc::Rc;

use wasm_bindgen::JsValue;

use crate::prelude::*;

/// An input field, with optional start and end icons, and an optional info
/// tooltip.
#[component]
pub fn Input(
	/// The ID of the input.
	#[prop(into, optional)]
	id: MaybeSignal<String>,
	/// Placeholder text for the input.
	#[prop(into, optional)]
	placeholder: MaybeSignal<String>,
	/// The type of input
	#[prop(into, default = "text".into())]
	r#type: MaybeSignal<String>,
	/// Whether the input is disabled.
	#[prop(into, default = false.into())]
	disabled: MaybeSignal<bool>,
	/// The ref to forward to the input
	#[prop(into, optional)]
	r#ref: Option<NodeRef<html::Input>>,
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
	/// A loading spinner, if any.
	#[prop(into, optional)]
	loading: MaybeSignal<bool>,
	/// The tooltip for the input.
	#[prop(into, optional)]
	info_tooltip: MaybeSignal<Option<String>>,
	/// Color variant of the input.
	#[prop(into, optional)]
	variant: MaybeSignal<SecondaryColorVariant>,
	/// Input event handler, if any
	#[prop(optional, default = Box::new(|_| ()))]
	on_input: Box<dyn FnMut(ev::Event)>,
	/// The initial value of the input.
	#[prop(into, optional)]
	value: MaybeSignal<String>,
	/// Additional class names to apply to the input, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let node_ref = r#ref.unwrap_or_else(|| create_node_ref::<html::Input>());

	view! {
		<div class=move || format!(
			"input fr-fs-ct row-card bg-secondary-{} {}",
			variant.get().as_css_name(),
			class.get()
		)>
			{move || info_tooltip
				.get()
				.map(move |content| {
					TooltipContainer(
						TooltipContainerProps {
							content: String::new(),
							label: None,
							disable_focus: false,
							icon_color: Color::default(),
							variant: variant.get(),
							class: String::new(),
							children: Rc::new(move || content.clone().into_view().into()),
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
									on_click: props.on_click.clone(),
									enable_pulse: props.enable_pulse,
									fill: props.fill,
								}
							)
					)
					.into_view()
			}
			{move || start_text.get()}
			{
				match value {
					MaybeSignal::Static(value) => {
						view! {
							<input
								id={move || id.get()}
								ref={node_ref}
								class="mx-md of-hidden txt-of-ellipsis"
								disabled={move || disabled.get()}
								placeholder={move || placeholder.get()}
								on:input=on_input
								value=move || value.clone()
								type=move || r#type.get() />
						}
					}
					MaybeSignal::Dynamic(value) => {
						view! {
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
								type=move || r#type.get() />
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
									on_click: props.on_click.clone(),
									enable_pulse: props.enable_pulse,
									fill: props.fill,
								}
							)
					)
					.into_view()
			}
			{move || loading.get().then(|| Spinner(SpinnerProps {
				class: String::from("spinner-xs").into(),
			}))}
		</div>
	}
}
