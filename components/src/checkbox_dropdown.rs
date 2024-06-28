use ev::MouseEvent;

use crate::{imports::*, input_dropdown};

/// Creates an input with options appearing in a dropdown
#[component]
pub fn CheckboxDropdown(
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
	#[prop(into, optional, default = vec!["".to_owned()].into())]
	value: RwSignal<Vec<String>>,
	/// The Event handler when user checks an option
	#[prop(optional, into, default = Callback::new(|(_, _)| {}))]
	on_select: Callback<(MouseEvent, String)>,
	/// Placeholder to show if value is empty
	#[prop(into, optional, default = "Type Here...".to_owned().into())]
	placeholder: MaybeSignal<String>,
	/// Whether the component is disabled or not
	#[prop(optional, into, default = false.into())]
	disabled: MaybeSignal<bool>,
	/// Whether the component is in loading state or not
	#[prop(optional, into, default = false.into())]
	loading: MaybeSignal<bool>,
	/// Whether to render an input, or a span masquerading as one
	#[prop(optional, into)]
	enable_input: MaybeSignal<bool>,
) -> impl IntoView {
	let show_dropdown = create_rw_signal(false);

	let outer_div_class = class.with(|cname| {
		format!(
			"fr-fs-ct br-sm row-card full-width pos-rel px-xl py-xxs input-dropdown bg-secondary-{} {} {}",
			variant.as_css_name(),
			cname,
			value.with_untracked(|val| {
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
	let store_placehoder = store_value(placeholder);
	let store_on_select = store_value(on_select);

	view! {
		<div on:click={handle_click} class={outer_div_class}>
			<Show
				when=move || !value.get().is_empty()
				fallback=move || view! {
					{store_placehoder.with_value(|placeholder| placeholder.get())}
				}
			>
				{value.get().len()} "selected"
			</Show>

			<Icon icon={IconType::ChevronDown} class="ml-auto" size={Size::ExtraSmall}/>

			<Show when={move || show_dropdown.get()}>
				<div class={dropdown_class.clone()}>
					<ul class="full-width full-height ofx-hidden ofy-auto fc-fs-fs">
						<For
							each={move || store_options.with_value(|opt| opt.get())}
							key={|state| state.label.clone()}
							let:child
						>
							<li
								on:click={
									let child = child.clone();
									move |ev| {
										ev.prevent_default();
										if value.get().iter().any(|e| e.to_owned() == child.id.clone()) {
											value.update(|val| val.retain(|x| x.to_owned() != child.id.clone()))
										} else {
											value.update(|val| val.push(child.id.clone()))
										}

										on_select.call((ev, child.label.clone()));
									}
								}
								class={"ul-light fr-fs-ct full-width br-bottom-sm row-card"}
							>
								<label
									html_for=""
									class="txt-left fr-fs-ct cursor-pointer full-width full-height px-xl py-sm"
								>
									<input type="checkbox" class="ml-md mr-sm checkbox-sm" checked={
										value.get().iter().any(|e| e.to_owned() == child.id.clone())
									} />
									<span>{child.label}</span>
								</label>
							</li>
						</For>
					</ul>
				</div>
			</Show>
		</div>
	}
}
