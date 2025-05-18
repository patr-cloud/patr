use crate::prelude::*;

/// A Extension of Input, to accommodate features specific to passwords
#[component]
pub fn PasswordInput(
	/// Name of the form control. Submitted with the form as part of a
	/// name/value pair
	#[props(into, optional)]
	name: String,
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
	let mut show_password = Signal::new(false);
	let input_type = Signal::memo(move || {
		if *show_password.read() {
			InputType::Text
		} else {
			InputType::Password
		}
	});

	let end_icon = rsx! {
		Icon {
			class: "text-white font-primary",
			color: Color::White,
			size: Size::Medium,
			onclick: move |_| {
			    show_password.toggle();
			},
			icon: if *show_password.read() { IconType::Eye } else { IconType::EyeOff },
		}
	};

	rsx! {
		Input {
			name,
			label,
			class,
			oninput,
			required,
			id,
			value,
			form,
			placeholder,
			disabled,
			r#type: *input_type.read(),
			start_icon,
			start_text,
			end_text,
			variant,
			end_icon,
		}
	}
}
