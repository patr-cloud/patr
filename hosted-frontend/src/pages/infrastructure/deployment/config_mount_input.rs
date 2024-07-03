use crate::prelude::*;

#[component]
pub fn ConfigMountInput(
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// List of all mount file names
	#[prop(into, optional, default = vec![].into())]
	mount_points: MaybeSignal<Vec<String>>,
) -> impl IntoView {
	let outer_div_class = class.with(|cname| format!("flex full-width {}", cname));
	let store_filenames = store_value(mount_points.clone());

	view! {
		<div class={outer_div_class}>
			<div class="flex-col-2 fr-fs-ct mb-auto mt-md">
				<label html_for="port" class="fr-fs-ct">
					"Config Mounts"
				</label>
			</div>

			<div class="flex-col-10 fc-fs-fs">
				<Show when={move || mount_points.with(|list| !list.is_empty())}>
					<div class="flex full-width">
						<div class="flex-col-12 fc-fc-fs">
							<For
								each={move || store_filenames.with_value(|list| list.get())}
								key={|state| state.clone()}
								let:filename
							>
								<div class="flex full-width mb-xs">
									<div class="flex-col-5 pr-lg">
										<div class="full-width full-height fr-fs-ct px-xl br-sm bg-secondary-light">
											<span class="ml-md txt-disabled">"/etc/config"</span>
											<span class="txt-of-ellipsis of-hidden w-20">
												{filename}
											</span>
										</div>
									</div>

									<div class="flex-col-6">
										<div class="full-width row-card fr-fs-ct px-xl py-sm br-sm bg-secondary-light">
											<span class="mx-md txt-of-ellipsis of-hidden w-45">
												"/etc/"
											</span>
										</div>
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
						<Input
							r#type={InputType::File}
							id="port"
							class="full-width"
							placeholder="No File Selected"
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
