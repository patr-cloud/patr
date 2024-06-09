use crate::imports::*;

/// The options to display in the dropdown
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct InputDropdownOption {
	/// The label of the option
	pub label: String,
	/// Whether it's checked by default or not
	pub disabled: bool,
}

/// Creates an input with options appearing in a dropdown
#[component]
pub fn InputDropdown(
	/// The List of options to display
	#[prop(into, optional, default = vec![].into())]
	options: MaybeSignal<Vec<InputDropdownOption>>,
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The color variant of the component
	#[prop(optional, default = SecondaryColorVariant::Light)]
	variant: SecondaryColorVariant,
	/// The default value of the input, if none is provided,
	/// defaults to empty string and placeholder is shown
	#[prop(into, optional, default = "".to_owned().into())]
	value: MaybeSignal<String>,
	/// Placeholder to show if value is empty
	#[prop(into, optional, default = "Type Here...".to_owned().into())]
	placeholder: MaybeSignal<String>,
	/// Whether the componenet is disabled or not
	#[prop(optional, into, default = false.into())]
	disabled: MaybeSignal<bool>,
	/// Whether the componenet is in loading state or not
	#[prop(optional, into, default = false.into())]
	loading: MaybeSignal<bool>,
	/// Whether to render an input, or a span masquerading as one
	#[prop(optional, into, default = false.into())]
	enable_input: MaybeSignal<bool>,
) -> impl IntoView {
	let show_dropdown = create_rw_signal(false);

	let outer_div_class = class.with(|cname| {
		format!(
			"fr-fs-ct br-sm row-card full-width pos-rel px-xl py-xxs input-dropdown bg-secondary-{} {} {}",
			variant.as_css_name(),
			cname,
			value.with(|val| {
				if val.is_empty() || disabled.get() || loading.get() {
					"txt-disabled"
				} else {
					"txt-white"
				}
			})
		)
	});

	let input_class = move || {
		format!(
			"full-width full-height txt-medium pl-sm mr-sm py-xxs br-sm {}",
			if disabled.get() {
				"txt-disabled"
			} else {
				"txt-white"
			}
		)
	};

	let dropdown_class = move || {
		format!(
			"pos-abs drop-down txt-white fc-fs-fs br-sm of-hidden full-width mt-lg bg-secondary-{} {}",
			variant.as_css_name(),
			class.get()
		)
	};

	let handle_click = move |_| {
		if !disabled.get() && !loading.get() {
			show_dropdown.update(|val| *val = !*val);
		}
	};

	let store_options = store_value(options);

	let input_value = create_rw_signal(value.get_untracked());
	let store_placehoder = store_value(placeholder.clone());

	let handle_click_option = move |state: &InputDropdownOption| {
		if state.disabled {
			show_dropdown.set(false);
		}

		input_value.set(state.label.clone());
	};

	view! {
		<div on:click={handle_click} class={outer_div_class}>
			<Show
				when={move || enable_input.get()}
				fallback={move || view! {
					<span class={input_class}>
						{
							if input_value.get().is_empty() {
								store_placehoder.with_value(|placeholder| placeholder.get().into_view())
							} else {
								input_value.get().into_view()
							}
						}
					</span>
				}}
			>

				<input
					r#type={InputType::Text.as_html_attribute()}
					placeholder={placeholder.clone()}
					disabled={disabled.get()}
					class={input_class}
					prop:value={input_value}
				/>
			</Show>
			<Icon icon={IconType::ChevronDown} class="ml-auto" size={Size::ExtraSmall}/>

			<Show when={move || show_dropdown.get()}>
				<div class={dropdown_class.clone()}>
					<ul class="full-width full-height ofx-hidden ofy-auto fc-fs-fs">
						<For
							each={move || store_options.with_value(|opt| opt.clone().get())}
							key={|state| state.clone()}
							let:child
						>
							<li
								on:click={
									let child = child.clone();
									move |_| handle_click_option(&child)
								}
								class={format!(
									"px-xl py-sm ul-light fr-fs-ct full-width br-bottom-sm {}",
									if child.clone().disabled { "txt-disabled" } else { "txt-white" },
								)}
							>

								{child.clone().label}
							</li>
						</For>
					</ul>
				</div>
			</Show>
		</div>
	}
}
