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
	let outer_div_class = class.with(|cname| format!("flex full-width {}", cname));
	let store_volumes = store_value(volumes_list.clone());

	let vol_path = create_rw_signal("".to_string());
	let vol_size = create_rw_signal("".to_string());
	view! {
		<div class={outer_div_class}>
			<div class="flex-col-2 fr-fs-ct mb-auto mt-md">
				<label html_for="port" class="fr-fs-ct">
					"Volumes"
				</label>
			</div>

			<div class="flex-col-10 fc-fs-fs">
				<Show when={move || volumes_list.with(|list| !list.is_empty())}>
					<div class="flex full-width">
						<div class="flex-col-12 fc-fs-fs">
							<For
								each={move || store_volumes.with_value(|list| list.get())}
								key={|state| state.clone()}
								let:vol
							>
								<div class="flex full-width mb-xs">
									<div class="flex-col-11 pr-lg">
										<div class="full-width fr-fs-ct px-xl py-sm br-sm bg-secondary-light">
											<span class="ml-md txt-of-ellipsis of-hidden-40">
												{vol.1}
											</span>
										</div>
									</div>

									<div class="flex-col-1 fr-ct-ct pl-sm">
										<button
											on:click={
												move |ev| {
													on_delete.call((ev, vol.0))
												}
											}
										>
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

				<div class="flex full-width">
					<div class="flex-col-5 fc-fc-fs pr-lg">
						<Input
							r#type={InputType::Text}
							id="volName"
							placeholder="Enter Volume Path"
							class="full-width"
							value={Signal::derive(move || vol_path.get())}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								vol_path.set(event_target_value(&ev))
							})}
						/>
					</div>

					<div class="flex-col-6 fc-fs-fs gap-xxs">
						<Input
							r#type={InputType::Text}
							id="envValue"
							placeholder="Enter Volume Size"
							end_text={Some("GB".to_string())}
							class="full-width"
							value={Signal::derive(move || vol_size.get())}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								vol_size.set(event_target_value(&ev))
							})}
						/>
					</div>

					<div class="flex-col-1 fr-ct-fs">
						<Link
							style_variant={LinkStyleVariant::Contained}
							class="br-sm p-xs ml-md"
							should_submit=false
							on_click={Rc::new(move |ev| {
								on_add.call((ev.clone(), vol_path.get(), vol_size.get()))
							})}
						>
							<Icon icon={IconType::Plus} color={Color::Secondary}/>
						</Link>
					</div>
				</div>
			</div>
		</div>
	}
}
