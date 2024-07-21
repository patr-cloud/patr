use leptos_use::{use_cookie, utils::FromToStringCodec};

use crate::{
	pages::{DeploymentInfo, MachineTypeCard},
	prelude::*,
};

#[component]
pub fn ScaleDeployment() -> impl IntoView {
	let deployment_info = expect_context::<RwSignal<DeploymentInfo>>();
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let machine_list = create_resource(
		move || current_workspace_id.get(),
		move |workspace_id| async move { list_all_machines(workspace_id).await },
	);

	view! {
		<div class="fc-fs-fs full-width px-xl mt-xl txt-white txt-sm fit-wide-screen mx-auto gap-md">
			<h4 class="txt-white txt-lg pb-md txt-white">"Scale Your Servers"</h4>

			<div class="flex full-width">
				<div class="flex-col-2 my-auto pr-md">
					<span class="txt-sm">"Choose Horizontal Scale"</span>
					<p class="full-width letter-sp-md mb-lg txt-xxs">
						"Choose the minimum and maximum number of instances for your deployment "
					</p>
				</div>

				<div class="flex-col-10 fc-fs-fs gap-sm px-md br-sm">
					<div
						style="width: 30%"
						class="full-width full-height fr-sb-ct"
					>
						<label
							class="flex-col-3"
							html_for="minHorizontalScale"
						>
							"Minimum Scale"
						</label>

						<input
							class="mx-md txt-white txt-center outline-primary-focus py-xxs br-sm bg-secondary-light flex-col-9 full-height"
							type="number"
							min={1}
							max={10}
							id="minHorizontalScale"
							name="min_horizontal_scale"
							value={1}
						/>
					</div>

					<div
						style="width: 30%;"
						class="full-width full-height fr-sb-ct"
					>
						<label
							class="flex-col-3"
							html_for="maxHorizontalScale"
						>
							"Maximum Scale"
						</label>

						<input
							class="mx-md txt-white txt-center outline-primary-focus py-xxs br-sm bg-secondary-light flex-col-9 full-height"
							type="number"
							min={1}
							max={10}
							id="maxHorizontalScale"
							name="max_horizontal_scale"
							value={1}
						/>
					</div>
				</div>
			</div>

			<div class="flex full-width">
				<div class="flex-col-2 my-auto pr-md">
					<span class="txt-sm">"Manage Resource Allocation"</span>

					<p class="letter-sp-md mb-lg txt-xxs">
						"Specify the resources to be allocated to your container"
					</p>
				</div>

				<div class="flex-col-10 fr-fs-ct of-auto">
					<div class="full-width p-xl br-sm fc-fs-fs of-auto">
						<Transition>
							<div class="fr-fs-ct f-wrap ofx-auto py-xxs gap-xl">
								{
									move || match machine_list.get() {
										Some(Ok(data)) => view! {
											<For
												each={move || data.clone().machine_types}
												key={|state| state.id.clone()}
												let:child
											>
												<MachineTypeCard
													machine_type={child}
													on_select={move |id: Uuid| {
														deployment_info.update(
															|info| info.machine_type = Some(id.to_string())
														)
													}}
												/>
											</For>
										}.into_view(),
										_ => "Couldn't load resource".into_view()
									}
								}
							</div>
						</Transition>
					</div>
				</div>
			</div>
		</div>
	}
}
