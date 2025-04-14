use std::rc::Rc;

use crate::imports::*;

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
	/// A Callender like date picker
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
	name: MaybeSignal<String>,
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: String,
	/// Specifies whether the form field needs to be filled in before it can
	/// be submitted, doesn't use javascript, defaults to false
	#[prop(into, optional, default = false.into())]
	required: bool,
	/// The Patter of the input, a string regex
	#[prop(into, optional)]
	_pattern: MaybeSignal<String>,
	/// The ID of the input.
	#[prop(into, optional)]
	id: MaybeSignal<String>,
	/// The form id of the input.
	#[prop(into, optional, default = None.into())]
	form: MaybeSignal<Option<String>>,
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
			"input flex justify-start items-center row-card bg-secondary-{} {}",
			variant.get().as_css_name(),
			class
		)
	};

	let end_icon = if r#type.with(|input_type| matches!(input_type, InputType::Password)) {
		MaybeSignal::derive(move || {
			Some(
				IconProps::builder()
					.icon(MaybeSignal::derive(move || {
						if show_password.get() {
							IconType::Eye
						} else {
							IconType::EyeOff
						}
					}))
					.size(
						end_icon
							.with_untracked(|props| props.as_ref().map(|props| props.size))
							.unwrap_or(MaybeSignal::Static(Size::ExtraSmall)),
					)
					.color(
						end_icon
							.with_untracked(|props| props.as_ref().map(|props| props.color))
							.unwrap_or(MaybeSignal::Static(Color::White)),
					)
					.on_click(Rc::new(move |_| {
						show_password.set(!show_password.get());
					}))
					.class(
						end_icon
							.with_untracked(|props| props.as_ref().map(|props| props.class.clone()))
							.unwrap_or_default(),
					)
					.enable_pulse(
						end_icon
							.with_untracked(|props| props.as_ref().map(|props| props.enable_pulse))
							.unwrap_or_default(),
					)
					.fill(
						end_icon
							.with_untracked(|props| props.as_ref().map(|props| props.fill))
							.unwrap_or_default(),
					)
					.build(),
			)
		})
	} else {
		end_icon
	};

	view! {
		<div class={class}>
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
				form={move || form.get()}
				id={move || id.get()}
				class="mx-md overflow-hidden text-ellipsis"
				name={move || name.get()}
				placeholder={move || placeholder.get()}
				disabled={move || disabled.get()}
				// pattern=move || pattern.get()
				required={required}
				on:input={on_input}
				prop:value={value}
				type={move || {
					if let InputType::Password = r#type.get() {
						if show_password.get() { InputType::Text } else { InputType::Password }
					} else {
						r#type.get()
					}
						.as_html_attribute()
				}}
			/>

			{move || end_text.get()}
			{move || {
				if let InputType::Password = r#type.get() {
					show_password_icon
						.get()
						.then(|| {
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
						})
						.into_view()
				} else {
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
			}}

		</div>
	}
}
