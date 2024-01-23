use std::rc::Rc;

use crate::imports::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum InputType {
	/// The default value. A single-line text field. Line-breaks are
	/// automatically removed from the input value.
	#[default]
	Text,
	/// A field for editing an email address. Looks like a text input, but has
	/// validation parameters and relevant keyboard in supporting browsers and
	/// devices with dynamic keyboards.
	Email,
	/// A single-line text field whose value is obscured. Will alert user if
	/// site is not secure.
	Password,
	/// A control for entering a telephone number. Displays a telephone keypad
	/// in some devices with dynamic keypads.
	Phone,
	/// A control for entering a number. Displays a spinner and adds default
	/// validation. Displays a numeric keypad in some devices with dynamic
	/// keypads.
	Number,
	/// A check box allowing single values to be selected/deselected.
	Checkbox,
}

impl InputType {
	pub const fn as_html_attribute(self) -> &'static str {
		match self {
			Self::Text => "text",
			Self::Email => "email",
			Self::Phone => "tel",
			Self::Number => "number",
			Self::Checkbox => "checkbox",
			Self::Password => "password",
		}
	}
}

#[component]
pub fn Input(
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: String,
	/// The ID of the input.
	#[prop(into, optional)]
	id: MaybeSignal<String>,
	/// Placeholder text for the input.
	#[prop(into, optional)]
	placeholder: MaybeSignal<String>,
	/// The type of input
	#[prop(into, optional, default = InputType::Text.into())]
	r#type: MaybeSignal<InputType>,
	/// Whether the input is disabled.
	#[prop(into, optional, default = false.into())]
	disabled: MaybeSignal<bool>,
	/// Input event handler
	#[prop(optional, default = Box::new(|_| ()))]
	on_input: Box<dyn FnMut(ev::Event)>,
	/// The Color Variant of the input
	#[prop(into, optional)]
	variant: MaybeSignal<SecondaryColorVariant>,
	/// Label for the input, an empty string doesn't render the label,
	/// defaults to empty string
	#[prop(into, optional, default = "".into())]
	label: MaybeSignal<String>,
	/// The Initial Value of the input
	#[prop(into, optional)]
	value: MaybeSignal<String>,
	/// The End Icon if any
	#[prop(into, optional)]
	end_icon: MaybeSignal<Option<IconProps>>,
	/// The End Text, if any
	#[prop(into, optional)]
	end_text: MaybeSignal<Option<String>>,
	/// The Start Icon if any
	#[prop(into, optional)]
	start_icon: MaybeSignal<Option<IconProps>>,
	/// The Start Text, if any
	#[prop(into, optional)]
	start_text: MaybeSignal<Option<String>>,
) -> impl IntoView {
	let show_password_icon = create_rw_signal(false);
	let show_password = create_rw_signal(false);

	create_effect(move |_| {
		show_password_icon.set(true);
	});

	let class = move || {
		format!(
			"input fr-fs-ct row-card bg-secondary-{} {}",
			variant.get().as_css_name(),
			class
		)
	};

	view! {
		<div class=class>
			<Show when={
				let label = label.clone();
				move || label.with(|lbl| !lbl.is_empty())
			}>
				<label>{label.get()}</label>
			</Show>
			{move || start_text.get()}

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
				})
				.into_view()}

			<input
				id=move || id.get()
				class="mx-md of-hidden txt-of-ellipsis"
				placeholder=move || placeholder.get()
				disabled=move || disabled.get()
				on:input=on_input
				value=move || value.clone()
				type=move || r#type.get().as_html_attribute()
			/>

			{move || end_text.get()}
			{show_password_icon
				.with(|&show_password_icon| {
					if show_password_icon {
						match r#type.get() {
							InputType::Password => {
								view! {
									<Show when=move || {
										show_password_icon
									}>
										{end_icon
											.with(|props| {
												props
													.as_ref()
													.map(|props| IconProps {
														icon: MaybeSignal::derive(move || if show_password.get() {
															IconType::Eye
														} else {
															IconType::EyeOff
														}),
														size: props.size,
														color: props.color,
														class: props.class.clone(),
														on_click: Some(Rc::new(move |_| show_password.set(!show_password.get()))),
														enable_pulse: props.enable_pulse,
														fill: props.fill,
													})
											})
											.into_view()}
									</Show>
								}
							}
							_ => {
								end_icon
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
									})
									.into_view()
							}
						}
					} else {
						Default::default()
					}
				})}

		</div>
	}
}
