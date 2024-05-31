use crate::prelude::*;

#[component]
pub fn EnvInput(
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// List of ports already present
	#[prop(into, optional, default = vec![].into())]
	envs_list: MaybeSignal<Vec<String>>,
) -> impl IntoView {
	let outer_div_class = class.with(|cname| format!("flex full-width {}", cname));
	let store_envs = store_value(envs_list.clone());

	view! {
		<div class={outer_div_class}>
			<div class="flex-col-2 fr-fs-ct mb-auto mt-md">
				<label html_for="port" class="fr-fs-ct">
					"Environment Variables"
				</label>
			</div>

			<div class="flex-col-10 fc-fs-fs">
				<Show when={move || envs_list.with(|list| !list.is_empty())}>
					<div class="flex full-width">
						<div class="flex-col-12 fc-fs-fs">
							<For
								each={move || store_envs.with_value(|list| list.get())}
								key={|state| state.clone()}
								let:child
							>
								<div class="flex full-width mb-xs">
									<div class="flex-col-5 pr-lg">
										<div class="full-width fr-fs-ct px-xl py-sm br-sm bg-secondary-light">
											<span class="ml-md txt-of-ellipsis of-hidden-40">
												{child}
											</span>
										</div>
									</div>

									<div class="flex-col-6">
										<Input
											disabled=false
											placeholder="Enter Env Value"
											class="full-width"
											value="https://123.123.123.123"
										/>
									</div>

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
					<div class="flex-col-5 fc-fc-fs pr-lg">
						<Input
							r#type={InputType::Text}
							id="envKey"
							placeholder="Enter Env Key"
							class="full-width"
						/>
					</div>

					<div class="flex-col-6 fc-fs-fs gap-xxs">
						<Input
							r#type={InputType::Text}
							id="envValue"
							placeholder="Enter Env Value"
							class="full-width"
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
