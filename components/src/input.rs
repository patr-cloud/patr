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
}

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
	#[prop(into, optional, default = false.into())]
	disabled: MaybeSignal<bool>,
	/// Additional class names to apply to the input, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Input event handler
	on_input: Box<dyn FnMut(ev::Event)>,
) -> impl IntoView {
	view! {
		<div>
			<label></label>
			<input
				id={move || id.get()}
				class="mx-md of-hidden txt-of-ellipsis"
				placeholder={move || placeholder.get()}
				disabled={move || disabled.get()}
                on:input=on_input
                // value=move || value.clone()
                type=move || r#type.get()
			/>
		</div>
	}
}
