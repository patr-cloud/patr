use std::{collections::BTreeMap, rc::Rc};

use ev::MouseEvent;
use models::api::workspace::deployment::EnvironmentVariableValue;

use crate::prelude::*;

#[component]
pub fn EnvInput(
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// List of ports already present
	#[prop(into, optional, default = BTreeMap::new().into())]
	envs_list: MaybeSignal<BTreeMap<String, EnvironmentVariableValue>>,
	/// On Pressing Delete Button
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_delete: Callback<String>,
	/// On Pressing Add Button
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_add: Callback<(String, String)>,
) -> impl IntoView {
	let outer_div_class = class.with(|cname| format!("flex full-width {}", cname));
	let store_envs = store_value(envs_list.clone());

	let env_name = create_rw_signal("".to_string());
	let env_value = create_rw_signal("".to_string());

	view! {
		<div class={outer_div_class}>
			<div class="flex-col-2 fr-fs-ct mb-auto mt-md">
				<label html_for="port" class="fr-fs-ct">
					"Environment Variables"
				</label>
			</div>

			<div class="flex-col-10 fc-fs-fs">
				<Show when={move || envs_list.with(|list| !list.is_empty())}>
					<div class="flex w-full">
						<div class="flex-col-12 fc-fs-fs">
							<For
								each={move || store_envs.with_value(|list| list.get())}
								key={|state| state.clone()}
								let:child
							>
								<div class="flex w-full mb-xs">
									<div class="flex-col-5 pr-lg">
										<div class="w-full fr-fs-ct px-xl py-sm br-sm bg-secondary-light">
											<span class="ml-md text-ellipsis of-hidden-40">
												{child.0.clone()}
											</span>
										</div>
									</div>

									<div class="flex-col-6">
										<div class="w-full fr-fs-ct px-xl py-sm bg-secondary-light br-sm">
											<span class="px-sm">{child.1.value()}</span>
										</div>
									</div>

									<div class="flex-col-1 fr-ct-ct pl-sm">
										<button on:click={move |ev| {
											on_delete.call(child.0.clone())
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
					<div class="flex-col-5 fc-fc-fs pr-lg">
						<Input
							r#type={InputType::Text}
							id="envKey"
							placeholder="Enter Env Key"
							class="w-full"
							value={Signal::derive(move || env_name.get())}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								env_name.set(event_target_value(&ev))
							})}
						/>
					</div>

					<div class="flex-col-6 fc-fs-fs gap-xxs">
						<Input
							r#type={InputType::Text}
							id="envValue"
							placeholder="Enter Env Value"
							class="w-full"
							value={Signal::derive(move || env_value.get())}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								env_value.set(event_target_value(&ev))
							})}
						/>
					</div>

					<div class="flex-col-1 fr-ct-fs">
						<Link
							style_variant={LinkStyleVariant::Contained}
							class="br-sm p-xs ml-md"
							should_submit=true
							on_click={Rc::new(move |ev| {
								ev.prevent_default();
								on_add.call((env_name.get(), env_value.get()))
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
