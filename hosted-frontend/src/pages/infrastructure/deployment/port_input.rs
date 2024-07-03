use std::collections::BTreeMap;

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
	on_delete: Callback<(MouseEvent, String)>,
	/// On Pressing Add Button
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_add: Callback<(MouseEvent, String, String)>,
	/// The Error For Port Input
	#[prop(into, optional)]
	error: MaybeSignal<String>,
) -> impl IntoView {
	let outer_div_class = class.with(|cname| format!("flex full-width {}", cname));

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
			<div class="flex-col-2 fr-fs-ct mb-auto mt-md">
				<label html_for="port" class="fr-fs-ct">
					"Ports"
				</label>
			</div>

			<div class="flex-col-10 fc-fs-fs">
				<Show when={move || ports_list.with(|list| !list.is_empty())}>
					<div class="flex full-width">
						<div class="flex-col-12 fc-fs-fs">
							<For
								each={move || store_ports.with_value(|list| list.get())}
								key={|state| state.clone()}
								let:child
							>
								<div class="flex full-width mb-xs">
									<div class={format!(
										"flex-col-{} pr-lg",
										if is_update_screen.get() { "2" } else { "5" },
									)}>
										<div class="full-width fr-fs-ct px-xl py-sm br-sm bg-secondary-light">
											<span class="ml-md">{child.0.to_string()}</span>
										</div>
									</div>

									<div class={(if is_update_screen.get() {
										"flex-col-3 pr-lg"
									} else {
										"flex-col-6"
									})
										.to_string()}>

										<div class="full-width fr-fs-ct px-xl py-sm bg-secondary-light br-sm">
											<span class="px-sm">{child.1.to_string()}</span>
										</div>
									</div>

									{is_update_screen
										.get()
										.then(|| {
											view! {
												<div class="flex-col-6 fr-ct-ct">
													<div class="bg-secondary-light br-sm py-sm px-xl full-width fr-fs-ct">
														<a
															href="https://onpatr.cloud"
															target="_blank"
															class="txt-underline ml-sm fr-fs-ct"
															rel="noreferrer"
														>
															<span class="txt-of-ellipsis of-hidden w-50">
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
										})
									}

									<div class="flex-col-1 fr-ct-ct pl-sm">
										<button
											on:click={
												move |ev| {
													on_delete.call((ev, child.0.to_string()))
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
					<div class="flex-col-5 fc-fs-fs pr-lg gap-xxs">
						<Input
							value={Signal::derive(move || port_number.get())}
							on_input={
								Box::new(move |ev| {
									ev.prevent_default();
									port_number.set(event_target_value(&ev));
								})
							}
							r#type={InputType::Number}
							id="port"
							class="full-width"
							placeholder="Enter Port Number"
						/>


						<Show when={move || store_error.with_value(|error| !error.get().clone().is_empty())}>
							<Alert r#type={AlertType::Error} class="mt-xs">
								{move || store_error.with_value(|error| error.get().clone())}
							</Alert>
						</Show>
					</div>

					<div class="flex-col-6 fc-fs-fs gap-xxs">
						<InputDropdown
							value={port_type}
							placeholder={"Select Protocol".to_string()}
							options={exposed_port_types}
						/>

					</div>

					<div class="flex-col-1 fr-ct-fs">
						<button
							on:click={move |ev| {
								on_add.call((ev, port_number.get(), port_type.get()))
							}}
							class="btn btn-primary br-sm p-xs ml-md"
							type="button"
						>
							<Icon icon={IconType::Plus} color={Color::Secondary}/>
						</button>
					</div>
				</div>
			</div>
		</div>
	}
}
