use std::{collections::BTreeMap, rc::Rc};

use ev::MouseEvent;

use crate::prelude::*;

#[component]
pub fn VolumeInput(
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// List of ports already present
	#[prop(into, optional, default = BTreeMap::new().into())]
	volumes_list: MaybeSignal<BTreeMap<Uuid, String>>,
	/// On Pressing Delete Button
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_delete: Callback<(MouseEvent, Uuid)>,
	/// On Pressing Add Button
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_add: Callback<(MouseEvent, String, String)>,
) -> impl IntoView {
	let outer_div_class = class.with(|cname| format!("flex w-full {}", cname));
	let store_volumes = store_value(volumes_list.clone());

	let vol_path = create_rw_signal("".to_string());
	let vol_size = create_rw_signal("".to_string());
	view! {
		<div class={outer_div_class}>
			<div class="flex-2 flex items-center justify-start mb-auto mt-md">
				<label html_for="port" class="flex justify-start items-center">
					"Volumes"
				</label>
			</div>

			<div class="flex-10 flex flex-col items-start justify-start">
				<Show when={move || volumes_list.with(|list| !list.is_empty())}>
					<div class="flex w-full">
						<div class="flex-12 flex flex-col items-start justify-start">
							<For
								each={move || store_volumes.with_value(|list| list.get())}
								key={|state| state.clone()}
								let:vol
							>
								<div class="flex w-full mb-xs">
									<div class="flex-11 pr-lg">
										<div class="w-full flex items-center justify-start px-xl py-sm br-sm bg-secondary-light">
											<span class="ml-md text-of-ellipsis of-hidden-40">
												{vol.1}
											</span>
										</div>
									</div>

									<div class="flex-1 flex items-center justify-center pl-sm">
										<button on:click={move |ev| {
											on_delete.call((ev, vol.0))
										}}>
											<Icon
												icon={IconType::Trash2}
												color={Color::Error}
												size={Size::Small}
											/>
										</button>
									</div>
								</div>
							</For>
						</div>
					</div>
				</Show>

				<div class="flex w-full">
					<div class="flex-5 flex flex-col items-start justify-center pr-lg">
						<Input
							r#type={InputType::Text}
							id="volName"
							placeholder="Enter Volume Path"
							class="w-full"
							value={Signal::derive(move || vol_path.get())}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								vol_path.set(event_target_value(&ev))
							})}
						/>
					</div>

					<div class="flex-6 flex flex-col items-start justify-start gap-xxs">
						<Input
							r#type={InputType::Text}
							id="envValue"
							placeholder="Enter Volume Size"
							end_text={Some("GB".to_string())}
							class="w-full"
							value={Signal::derive(move || vol_size.get())}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								vol_size.set(event_target_value(&ev))
							})}
						/>
					</div>

					<div class="flex-1 flex items-start justify-center">
						<Link
							style_variant={LinkStyleVariant::Contained}
							class="br-sm p-xs ml-md"
							should_submit=false
							on_click={Rc::new(move |ev| {
								on_add.call((ev.clone(), vol_path.get(), vol_size.get()))
							})}
						>
							<Icon icon={IconType::Plus} color={Color::Secondary} />
						</Link>
					</div>
				</div>
			</div>
		</div>
	}
}
