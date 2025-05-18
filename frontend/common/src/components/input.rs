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

/// Wraps around HTML input, with additional props for styling and such
#[component]
pub fn Input(
	/// Name of the form control. Submitted with the form as part of a
	/// name/value pair
	#[props(into, optional)]
	name: String,
	/// The type of input
	#[props(into, optional, default = InputType::Text)]
	r#type: InputType,
	/// Input event handler
	#[props(optional, into, default = EventHandler::new(|_| {}))]
	oninput: EventHandler<Event<FormData>>,
	/// Additional class names to apply to the outer div, if any.
	#[props(into, optional)]
	class: String,
	/// Specifies whether the form field needs to be filled in before it can
	/// be submitted, doesn't use javascript, defaults to false
	#[props(into, optional, default = false)]
	required: bool,
	/// The ID of the input.
	#[props(into, optional)]
	id: String,
	/// The form id of the input.
	#[props(into, optional, default = None)]
	form: Option<String>,
	/// Placeholder text for the input.
	#[props(into, optional)]
	placeholder: String,
	/// Whether the input is disabled.
	#[props(into, optional, default = false)]
	disabled: bool,
	/// The Color Variant of the input
	#[props(into, optional)]
	variant: SecondaryColorVariant,
	/// Label for the input, an empty string doesn't render the label,
	/// defaults to empty string
	#[props(into, optional, default = "")]
	label: String,
	/// The Initial Value of the input
	#[props(into, optional)]
	value: String,
	/// The End Icon if any
	#[props(into, optional, default = VNode::empty())]
	end_icon: Element,
	/// The End Text, if any
	#[props(into, optional)]
	end_text: Option<String>,
	/// The Start Icon if any
	#[props(into, optional, default = VNode::empty())]
	start_icon: Element,
	/// The Start Text, if any
	#[props(into, optional)]
	start_text: Option<String>,
) -> Element {
	rsx! {
		div { class: "input flex justify-start items-center row-card bg-secondary-{variant.as_css_name()} {class}",
			if !label.is_empty() {
				label { {label} }
			}

			{start_text}
			{start_icon}

			input {
				form,
				id,
				class: "mx-md overflow-hidden text-ellipsis",
				r#type: r#type.as_html_attribute(),
				name,
				placeholder,
				disabled,
				required,
				value,
				oninput: move |e| oninput.call(e),
			}

			{end_text}
			{end_icon}
		}
	}
}
