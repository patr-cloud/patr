use ev::MouseEvent;

use crate::imports::*;

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
	#[prop(into)]
	value: Signal<Vec<String>>,
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
	/// Additional View to add at the end of the list
	#[prop(optional, default = None)]
	additional_view: Option<View>,
	/// Additional Classes for the list item surrounding the action view
	#[prop(into, optional, default = "".to_string().into())]
	additional_view_class: MaybeSignal<String>,
	/// Message to display when there's no item in the list
	#[prop(into, optional, default = "No Item in the List".to_string().into())]
	empty_fallback: MaybeSignal<String>,
) -> impl IntoView {
	let show_dropdown = create_rw_signal(false);

	let outer_div_class = class.with(|cname| {
		format!(
			"flex justify-start items-center br-sm row-card w-full relative px-xl py-xxs input-dropdown bg-secondary-{} {} {}",
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

	let css_class = "absolute drop-down text-white flex flex-col items-start justify-start rounded-sm overflow-hidden w-full mt-lg";
	let dropdown_class = move || {
		format!(
			"{} bg-secondary-{} {}",
			css_class,
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
	let store_empty_fallback = store_value(empty_fallback);
	let store_additional_view_class = store_value(additional_view_class);

	view! {
		<div on:click={handle_click} class={outer_div_class}>
			<Show
				when=move || !value.get().is_empty()
				fallback=move || view! {
					{store_placehoder.with_value(|placeholder| placeholder.get())}
				}
			>
				{value.get().len()}" Selected"
			</Show>

			<Icon icon={IconType::ChevronDown} class="ml-auto" size={Size::ExtraSmall}/>

			<Show when={move || show_dropdown.get()}>
				<div class={dropdown_class.clone()}>
					<ul class="w-full h-full overflow-x-hidden overflow-y-auto flex flex-col items-start justify-start">
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
										on_select.call((ev, child.id.clone()));
									}
								}
								class={"flex justify-start items-center w-full row-card border-border-color border-b-2 br-bottom-sm"}
							>
								<label
									html_for=""
									class="text-left flex justify-start items-center cursor-pointer w-full h-full px-xl py-sm"
								>
									<input
										type="checkbox"
										class="ml-md mr-sm checkbox-sm"
										prop:checked={
											value.get().iter().any(|e| *e == child.id)
										}
									/>
									<span>{child.label}</span>
								</label>
							</li>
						</For>

						<Show
							when={move || store_options.with_value(|opt| opt.get().len() == 0)}
						>
							<li
								class={"px-xl py-sm border-border-color border-b-2 bg-[#292548]
									flex justify-start items-center w-full br-bottom-sm text-disabled"
								}
							>
								{store_empty_fallback.with_value(|val| val.get())}
							</li>
						</Show>

						{
							additional_view.clone().map(|additional_view| {
								view! {
									<li
										class={move ||
											format!(
												"px-xl py-sm flex justify-start items-center
												border-border-color border-b-2 w-full br-bottom-sm {}",
												store_additional_view_class.with_value(|val| val.get())
											)
										}
									>
										{additional_view}
									</li>
								}
							})
						}
					</ul>
				</div>
			</Show>
		</div>
	}
}
