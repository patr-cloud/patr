use leptos::ev::Event;

use crate::prelude::*;

/// A Extension of Input, to accommodate features specific to passwords
#[component]
pub fn PasswordInput(
	/// Name of the form control. Submitted with the form as part of a
	/// name/value pair
	#[prop(into, optional)]
	name: Signal<String>,
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
	let show_password = RwSignal::new(false);
	let input_type = Signal::derive(move || {
		if show_password.get() {
			InputType::Text
		} else {
			InputType::Password
		}
	});

	view! {
		<Input
			name={name}
			label={label}
			class={class}
			on_input={on_input}
			required={required}
			id={id}
			value={value}
			form={form}
			placeholder={placeholder}
			disabled={disabled}
			r#type={input_type}
			start_icon={start_icon}
			start_text={start_text}
			end_text={end_text}
			variant={variant}
			end_icon={move || view! {
				<Icon
					class="text-white font-primary"
					color={Color::White}
					size={Size::Medium}
					on_click={move |_| {
						show_password.update(|val| *val = !*val);
					}}
					icon={ if show_password.get() {IconType::Eye} else {IconType::EyeOff}}
				/>
			}}
		/>
	}
}
