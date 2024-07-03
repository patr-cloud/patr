use leptos_use::{use_cookie, utils::FromToStringCodec};

use crate::{
	pages::{DeploymentInfo, MachineTypeCard},
	prelude::*,
};

#[component]
pub fn ScaleDeployment() -> impl IntoView {
	let min_horizontal = create_rw_signal::<u16>(2);
	let max_horizontal = create_rw_signal::<u16>(10);

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
				</div>

				<div class="flex-col-10 fc-fs-ct bg-secondary-light p-xl br-sm">
					<p class="full-width letter-sp-md mb-lg txt-xxs">
						"Choose the minimum and maximum number of instances for your deployment "
					</p>

					<div class="full-width fr-ct-ct">
						<div class="flex-col-2 fc-ct-ct">
							<label html_for="minHorizontalScale">"Minimum Scale"</label>

							<NumberPicker
								value={min_horizontal}
								style_variant={SecondaryColorVariant::Medium}
								on_change={move |_| {
									deployment_info.update(|info| {
										info.min_horizontal_scale = Some(min_horizontal.get());
									})
								}}
							/>
						</div>

						<div class="flex-col-8 mt-xl px-xl fc-fs-ct">
							<DoubleInputSlider
								min={min_horizontal}
								max={max_horizontal}
								min_limit={1}
								max_limit={10}
								class="full-width"
							/>

							<p class="txt-warning txt-xxs">
								"Any excess volumes will be removed if the number of instances is reduced."
							</p>
						</div>

						<div class="flex-col-2 fc-ct-ct">
							<label html_for="maxHorizontalScale">"Maximum Scale"</label>

							<NumberPicker
								value={max_horizontal}
								style_variant={SecondaryColorVariant::Medium}
								on_change={move |_| {
									deployment_info.update(|info| {
										info.max_horizontal_scale = Some(max_horizontal.get());
									})
								}}
							/>
						</div>
					</div>
				</div>
			</div>

			<div class="flex full-width">
				<div class="flex-col-2 my-auto pr-md">
					<span class="txt-sm">"Manage Resource Allocation"</span>
				</div>

				<div class="flex-col-10 fr-fs-ct of-auto">
					<div class="full-width p-xl br-sm bg-secondary-light fc-fs-fs of-auto">
						<p class="letter-sp-md mb-lg txt-xxs">
							"Specify the resources to be allocated to your container"
						</p>

						<Transition>
							<div class="fr-fs-ct ofx-auto py-xxs gap-md">
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
