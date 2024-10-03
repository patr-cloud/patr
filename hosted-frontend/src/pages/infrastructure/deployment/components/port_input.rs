use std::{collections::BTreeMap, rc::Rc};

use ev::MouseEvent;
use models::api::workspace::deployment::ExposedPortType;
use strum::VariantNames;

use crate::prelude::*;

#[component]
pub fn PortInput(
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// List of ports already present
	#[prop(into, optional, default = BTreeMap::new().into())]
	ports_list: MaybeSignal<BTreeMap<StringifiedU16, ExposedPortType>>,
	/// Whether updating, or viewing details.
	#[prop(into, optional, default = false.into())]
	is_update_screen: MaybeSignal<bool>,
	/// On Pressing Delete Button
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_delete: Callback<String>,
	/// On Pressing Add Button
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_add: Callback<(String, String)>,
	/// The Error For Port Input
	#[prop(into, optional)]
	error: MaybeSignal<String>,
) -> impl IntoView {
	let outer_div_class = class.with(|cname| format!("flex w-full {}", cname));

	let store_ports = store_value(ports_list.clone());
	let exposed_port_types = ExposedPortType::VARIANTS
		.iter()
		.map(|x| InputDropdownOption {
			id: x.to_string(),
			label: x.to_string(),
			disabled: false,
		})
		.collect::<Vec<_>>();

	let port_type = create_rw_signal("".to_string());
	let port_number = create_rw_signal("".to_string());

	let store_error = store_value(error);

	view! {
		<div class={outer_div_class}>
			<div class="flex-2 flex justify-start items-center mb-auto mt-md">
				<label html_for="port" class="flex justify-start items-center">
					"Ports"
				</label>
			</div>

			<div class="flex-10 flex flex-col items-start justify-start">
				<Show when={move || ports_list.with(|list| !list.is_empty())}>
					<div class="flex w-full">
						<div class="flex-12 flex flex-col items-start justify-start">
							<For
								each={move || store_ports.with_value(|list| list.get())}
								key={|state| state.clone()}
								let:child
							>
								<div class="flex w-full mb-xs">
									<div class={format!(
										"flex-{} pr-lg",
										if is_update_screen.get() { "2" } else { "5" },
									)}>
										<div class="w-full flex justify-start items-center px-xl py-sm br-sm bg-secondary-light">
											<span class="ml-md">{child.0.to_string()}</span>
										</div>
									</div>

									<div class={(if is_update_screen.get() {
										"flex-3 pr-lg"
									} else {
										"flex-6"
									})
										.to_string()}>

										<div class="w-full flex justify-start items-center px-xl py-sm bg-secondary-light br-sm">
											<span class="px-sm">{child.1.to_string()}</span>
										</div>
									</div>

									{is_update_screen
										.get()
										.then(|| {
											view! {
												<div class="flex-6 flex items-center justify-center">
													<div class="bg-secondary-light rounded-sm py-sm px-xl w-full flex justify-start items-center">
														<a
															href="https://onpatr.cloud"
															target="_blank"
															class="underline ml-sm w-full flex justify-between items-center"
															rel="noreferrer"
														>
															<span class="text-ellipsis overflow-hidden w-[50ch]">
																"https://onpatr.cloud"
															</span>
															<Icon
																icon={IconType::ExternalLink}
																size={Size::ExtraSmall}
																class="ml-xxs"
															/>
														</a>
													</div>
												</div>
											}
										})}

									<div class="flex-1 flex items-center justify-center pl-sm">
										<button on:click={move |ev| {
											on_delete.call(child.0.to_string())
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

				<form class="flex w-full">
					<div class="flex-5 flex flex-col justify-start items-start pr-lg gap-xxs">
						<Input
							value={Signal::derive(move || port_number.get())}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								port_number.set(event_target_value(&ev));
							})}
							r#type={InputType::Number}
							id="port"
							class="w-full"
							placeholder="Enter Port Number"
						/>

						<Show when={move || {
							store_error.with_value(|error| !error.get().clone().is_empty())
						}}>
							<Alert r#type={AlertType::Error} class="mt-xs">
								{move || store_error.with_value(|error| error.get().clone())}
							</Alert>
						</Show>
					</div>

					<div class="flex-6 flex flex-col items-start justify-start gap-xxs">
						<InputDropdown
							value={port_type}
							placeholder={"Select Protocol".to_string()}
							options={exposed_port_types}
							on_select={move |val: String| {
								port_type.set(val);
								if !port_type.get().is_empty() && !port_number.get().is_empty() {
									on_add.call((port_number.get(), port_type.get()));
								}
							}}
						/>

					</div>

					<div class="flex-1 flex items-start justify-center">
						<Link
							style_variant={LinkStyleVariant::Contained}
							class="br-sm p-xs ml-md"
							should_submit=true
							on_click={Rc::new(move |_| {
								on_add.call((port_number.get(), port_type.get()))
							})}
						>
							<Icon icon={IconType::Plus} color={Color::Secondary} />
						</Link>
					</div>
				</form>
			</div>
		</div>
	}
}
