use crate::prelude::*;

#[component]
pub fn PortInput(
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// List of ports already present
	#[prop(into, optional, default = vec![].into())]
	ports_list: MaybeSignal<Vec<String>>,
	/// Whether updating, or viewing details.
	#[prop(into, optional, default = false.into())]
	is_update_screen: MaybeSignal<bool>,
) -> impl IntoView {
	let outer_div_class = class.with(|cname| format!("flex full-width {}", cname));

	let store_ports = store_value(ports_list.clone());
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
											<span class="ml-md">{child}</span>
										</div>
									</div>

									<div class={(if is_update_screen.get() {
											"flex-col-3 pr-lg"
										} else {
											"flex-col-6"
										}).to_string()}>

										<div class="full-width fr-fs-ct px-xl py-sm bg-secondary-light br-sm">
											<span class="px-sm">"HTTP"</span>
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
										})}

									<div class="flex-col-1 fr-ct-ct pl-sm">
										<button>
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

				<form class="flex full-width">
					<div class="flex-col-5 fc-fs-fs pr-lg gap-xxs">
						<Input
							r#type={InputType::Number}
							id="port"
							class="full-width"
							placeholder="Enter Port Number"
						/>
					</div>

					<div class="flex-col-6 fc-fs-fs gap-xxs">
						<InputDropdown
							placeholder={"Select Protocol".to_string()}
							value="8080"
							options={vec![
								InputDropdownOption {
									label: "8080".to_owned(),
									disabled: false,
								},
							]}
						/>

					</div>

					<div class="flex-col-1 fr-ct-fs">
						<Link
							style_variant={LinkStyleVariant::Contained}
							class="br-sm p-xs ml-md"
							should_submit=true
						>
							<Icon icon={IconType::Plus} color={Color::Secondary}/>
						</Link>
					</div>
				</form>
			</div>
		</div>
	}
}
