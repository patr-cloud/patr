use ev::KeyboardEvent;

use crate::prelude::*;

/// The options to display in the dropdown
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct InputDropdownOption {
	/// The Id of the option
	pub id: String,
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
	/// On Selecting an Input
	#[prop(into, optional, default = Callback::new(|_| {}))]
	on_select: Callback<String>,
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The color variant of the component
	#[prop(optional, default = SecondaryColorVariant::Light)]
	variant: SecondaryColorVariant,
	/// The default value of the input, if none is provided,
	/// defaults to empty string and placeholder is shown
	#[prop(into, optional, default = "".to_owned().into())]
	value: RwSignal<String>,
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
			"flex justify-center items-center br-sm row-card w-full relative px-xl py-xxs 
			input-dropdown bg-secondary-{} {} {}",
			variant.as_css_name(),
			cname,
			value.with_untracked(|val| {
				if val.is_empty() || disabled.get() || loading.get() {
					"text-disabled"
				} else {
					"text-white"
				}
			})
		)
	});

	let dropdown_class = move || {
		format!(
			"absolute drop-down text-white br-sm overflow-hidden
			flex flex-col items-start justify-start w-full mt-lg bg-secondary-{} {}",
			variant.as_css_name(),
			class.get()
		)
	};

	let handle_toggle_options = move |_| {
		if !disabled.get() && !loading.get() {
			show_dropdown.update(|val| *val = !*val);
		}
	};

	let store_options = store_value(options);
	let store_placehoder = store_value(placeholder.clone());

	let input_class = move || {
		format!(
			"w-full h-full font-medium pl-sm mr-sm py-xxs br-sm {}",
			if disabled.get() ||
				(value.with(|val| val.is_empty()) &&
					store_placehoder.with_value(|placeholder| !placeholder.get().is_empty()))
			{
				"text-disabled"
			} else {
				"text-white"
			}
		)
	};

	let label = create_memo(move |_| {
		store_options.with_value(|options| {
			options
				.get()
				.clone()
				.into_iter()
				.find(|op| op.id == value.get())
				.map(|val| val.label)
				.unwrap_or_default()
		})
	});
	let handle_click_option = move |state: &InputDropdownOption| {
		if state.disabled {
			show_dropdown.set(false);
			return;
		}

		value.set(state.id.clone());
		on_select.call(state.id.clone());
	};

	let handle_keydown_input = move |e: KeyboardEvent| {
		e.stop_propagation();
		if !disabled.get() && !loading.get() {
			if e.key() == "Enter" || e.key() == "Space" {
				show_dropdown.update(|val| *val = !*val);
			}
		}
	};

	let handle_keydown_option = move |e: KeyboardEvent, child: &InputDropdownOption| {
		e.stop_propagation();
		if e.key() == "Enter" || e.key() == "Space" {
			handle_click_option(child);
			show_dropdown.set(false);
		}
	};

	view! {
		<div
			tabindex={0}
			on:click={handle_toggle_options}
			on:keydown={handle_keydown_input}
			class={outer_div_class}
		>
			<Show
				when={move || enable_input.get()}
				fallback={move || view! {
					<span class={input_class}>
						{
							if value.get().is_empty() || disabled.get() {
								store_placehoder.with_value(|placeholder| placeholder.get().into_view())
							} else {
								label.get().into_view()
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
					prop:value={label}
				/>
			</Show>
			<Icon icon={IconType::ChevronDown} class="ml-auto" size={Size::ExtraSmall}/>

			<Show when={move || show_dropdown.get() && !disabled.get()}>
				<div class={dropdown_class.clone()}>
					<ul class="w-full h-full overflow-x-hidden overflow-y-auto flex flex-col items-start justify-start">
						<For
							each={move || store_options.with_value(|opt| opt.clone().get())}
							key={|state| state.clone()}
							let:child
						>
							<li
								tabindex={0}
								on:click={
									let child = child.clone();
									move |_| handle_click_option(&child)

								}
								on:keydown={
									let child = child.clone();
									move |ev| handle_keydown_option(ev, &child)
								}
								class={format!(
									"px-xl py-sm flex justify-start items-center
									border-border-color border-b-2 w-full br-bottom-sm {}",
									if child.clone().disabled { "text-disabled" } else { "text-white" },
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
