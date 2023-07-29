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
	#[prop(into, optional)] class: MaybeSignal<String>,
) -> impl IntoView {
	let node_ref = ref_.unwrap_or_else(|| create_node_ref::<html::Input>(cx));

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
			{match start_icon {
				MaybeSignal::Static(Some(value)) => {
					let IconProps {
						icon,
						size,
						color,
						class,
						click,
						enable_pulse,
						fill,
					} = value;
					Icon(cx, IconProps {
						icon,
						size,
						color,
						class,
						click,
						enable_pulse,
						fill,
					}).into_view(cx)
				},
				MaybeSignal::Static(None) => ().into_view(cx),
				MaybeSignal::Dynamic(value) => {
					value.with(|props| {
						if let Some(props) = props {
						let IconProps {
							icon,
							size,
							color,
							class,
							click,
							enable_pulse,
							fill,
						} = props;
						Icon(cx, IconProps {
							icon: icon.clone(),
							size: size.clone(),
							color: color.clone(),
							class: class.clone(),
							click: click.take(),
							enable_pulse: enable_pulse.clone(),
							fill: fill.clone(),
						}).into_view(cx)
					} else {
						().into_view(cx)
					}})
				},
			}}
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
								value=value
								type_=move || type_.get() />
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
								type_=move || type_.get() />
						}
					}
				}
			}
			{move || end_text.get()}
			{move || end_icon.with(|props| props.map(|icon| Icon(cx, icon)))}
		</div>
	}
}
