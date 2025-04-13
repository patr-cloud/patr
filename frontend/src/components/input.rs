use leptos::ev::Event;

use crate::prelude::*;

/// The Type of the input
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
	/// An input which allows for the uploading of a file. Will be rendered as
	/// a button with a file picker dialog.
	File,
	/// A Calender like date picker
	Date,
	/// Hidden input, doesn't render on the dom, but it's name field
	/// will still be accessed by the _Ancestor Form Element_.
	/// Can be used to pass the id, or some other request data.
	Hidden,
}

impl InputType {
	/// Converts the enum into the corresponding html attribute string
	pub const fn as_html_attribute(self) -> &'static str {
		match self {
			Self::Text => "text",
			Self::Email => "email",
			Self::Phone => "tel",
			Self::Number => "number",
			Self::Checkbox => "checkbox",
			Self::Password => "password",
			Self::File => "file",
			Self::Date => "date",
			Self::Hidden => "hidden",
		}
	}
}

#[component]
pub fn Input(
	/// Name of the form control. Submitted with the form as part of a
	/// name/value pair
	#[prop(into, optional)]
	name: Signal<String>,
	/// The type of input
	#[prop(into, optional, default = InputType::Text.into())]
	r#type: Signal<InputType>,
	/// Input event handler
	#[prop(optional, into, default = UnsyncCallback::new(|_| {}))]
	on_input: UnsyncCallback<(Event,)>,
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: String,
	/// Specifies whether the form field needs to be filled in before it can
	/// be submitted, doesn't use javascript, defaults to false
	#[prop(into, optional, default = false.into())]
	required: bool,
	/// The ID of the input.
	#[prop(into, optional)]
	id: Signal<String>,
	/// The form id of the input.
	#[prop(into, optional, default = None.into())]
	form: Signal<Option<String>>,
	/// Placeholder text for the input.
	#[prop(into, optional)]
	placeholder: Signal<String>,
	/// Whether the input is disabled.
	#[prop(into, optional, default = false.into())]
	disabled: Signal<bool>,
	/// The Color Variant of the input
	#[prop(into, optional)]
	variant: Signal<SecondaryColorVariant>,
	/// Label for the input, an empty string doesn't render the label,
	/// defaults to empty string
	#[prop(into, optional, default = "".into())]
	label: Signal<String>,
	/// The Initial Value of the input
	#[prop(into, optional)]
	value: Signal<String>,
	/// The End Icon if any
	#[prop(into, optional)]
	end_icon: ViewFn,
	/// The End Text, if any
	#[prop(into, optional)]
	end_text: Signal<Option<String>>,
	/// The Start Icon if any
	#[prop(into, optional)]
	start_icon: ViewFn,
	/// The Start Text, if any
	#[prop(into, optional)]
	start_text: Signal<Option<String>>,
) -> impl IntoView {
	let class = move || {
		format!(
			"input flex justify-start items-center row-card bg-secondary-{} {}",
			variant.get().as_css_name(),
			class
		)
	};

	view! {
		<div class={class}>
			<Show when={
				move || label.with(|lbl| !lbl.is_empty())
			}>
				<label>{move || label.get()}</label>
			</Show>
			{move || start_text.get()}
			{move || start_icon.run()}
			<input
				form={move || form.get()}
				id={move || id.get()}
				class="mx-md overflow-hidden text-ellipsis"
				type={move || r#type.read().as_html_attribute()}
				name={move || name.get()}
				placeholder={move || placeholder.get()}
				disabled={move || disabled.get()}
				required={required}
				on:input={move |e| {
					on_input.run((e,))
				}}
				prop:value={value}
			/>
			{move || end_text.get()}
			{move || end_icon.run()}
		</div>
	}
}
