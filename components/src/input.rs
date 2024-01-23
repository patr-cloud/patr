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
	/// Additional classnames to apply to the outer div, if any.
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
	label: String,
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
	let cloned_label = label.clone();
	let show_label = move || !cloned_label.is_empty();

	let show_password = create_rw_signal(false);
	let is_js_enable = create_rw_signal(false);

	// FIX: use create_effect rather render_effect.
	create_render_effect(move |_| {
		is_js_enable.set(true);
		// log(format!("{:?}", is_js_enable))
	});

	let class = move || {
		format!(
			"input fr-fs-ct row-card bg-secondary-{} {}",
			variant.get().as_css_name(),
			class
		)
	};

	let end_icon_view = end_icon.with(|props| {
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
			.into_view()
	});

	let end_icon_view_cloned = end_icon_view.clone();
	// logging::log!("{:?}", is_js_enable.get());

	let password_icon = || match r#type.get() {
		InputType::Password => view! {
			<Show when=move || is_js_enable.get()>
				{ end_icon_view_cloned }
			</Show>
		},
		_ => end_icon
			.with(|props| {
				props.as_ref().map(|props| IconProps {
					icon: props.icon,
					size: props.size,
					color: props.color,
					class: props.class.clone(),
					on_click: props.on_click.clone(),
					enable_pulse: props.enable_pulse,
					fill: props.fill,
				})
			})
			.into_view(),
	};

	view! {
		<div class={class}>
			<Show when=show_label>
				<label>{label.clone()}</label>
			</Show>
			{move || start_text.get()}
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
			<input
				id={move || id.get()}
				class="mx-md of-hidden txt-of-ellipsis"
				placeholder={move || placeholder.get()}
				disabled={move || disabled.get()}
				on:input=on_input
				value={move|| value.clone()}
				type={move || r#type.get().as_html_attribute()}
			/>

			{move || end_text.get()}
			{password_icon}
			// <Show when=move || is_js_enable.get()>
			//     {
			//         end_icon
			//             .with(|props|
			//                 props
			//                     .as_ref()
			//                     .map(|props|
			//                         IconProps {
			//                             icon: props.icon,
			//                             size: props.size,
			//                             color: props.color,
			//                             class: props.class.clone(),
			//                             on_click: props.on_click.clone(),
			//                             enable_pulse: props.enable_pulse,
			//                             fill: props.fill,
			//                         }
			//                     )
			//             )
			//             .into_view()
			//     }
			// </Show>
		</div>
	}
}
