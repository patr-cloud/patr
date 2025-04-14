use super::super::components::*;
use crate::{pages::DeploymentInfo, prelude::*, queries::list_machines_query};

/// A component that allows the user to scale their deployment
#[component]
pub fn ScaleDeployment() -> impl IntoView {
	let min_horizontal = create_rw_signal::<u16>(2);
	let max_horizontal = create_rw_signal::<u16>(10);

	let deployment_info = expect_context::<RwSignal<DeploymentInfo>>();

	let machine_list = list_machines_query();

	view! {
		<div class="fc-fs-fs w-full px-xl mt-xl text-white text-sm fit-wide-screen mx-auto gap-md">
			<h4 class="text-white text-lg pb-md">"Scale Your Servers"</h4>

			<div class="flex w-full">
				<div class="flex-2 my-auto pr-md">
					<span class="text-sm">"Choose Horizontal Scale"</span>
				</div>

				<div class="flex-10 flex flex-col justify-start items-start bg-secondary-light p-xl br-sm gap-md">
					<p class="w-full tracking-[1px] text-xxs">
						"Choose the minimum and maximum number of instances for your deployment "
					</p>

					<div class="flex flex-col justify-start items-start gap-xl">
						<div
							style="width: 30%"
							class="w-full h-full flex justify-between items-center gap-xl"
						>
							<label class="flex-3" html_for="minHorizontalScale">
								"Minimum Scale"
							</label>

							<NumberPicker
								value={min_horizontal}
								style_variant={SecondaryColorVariant::Medium}
								on_change={move |_| {
									deployment_info
										.update(|info| {
											info.min_horizontal_scale = Some(min_horizontal.get());
										})
								}}
							/>
						</div>

						<div
							style="width: 30%;"
							class="w-full h-full flex justify-between items-center gap-xl"
						>
							<label class="flex-3" html_for="maxHorizontalScale">
								"Maximum Scale"
							</label>

							<NumberPicker
								value={max_horizontal}
								style_variant={SecondaryColorVariant::Medium}
								on_change={move |_| {
									deployment_info
										.update(|info| {
											info.max_horizontal_scale = Some(max_horizontal.get());
										})
								}}
							/>
						</div>
					</div>
				</div>
			</div>

			<div class="flex w-full">
				<div class="flex-2 my-auto pr-md">
					<span class="text-sm">"Manage Resource Allocation"</span>
				</div>

				<div class="flex-10 flex justify-start items-center overflow-auto">
					<div class="w-full p-xl rounded-sm bg-secondary-light flex flex-col items-start justify-start overflow-auto">
						<p class="tracking-[1px] mb-lg text-xxs">
							"Specify the resources to be allocated to your container"
						</p>

						<Transition>
							<div class="flex justify-start items-center overflow-x-auto py-xxs gap-md">
								{move || match machine_list.get() {
									Some(Ok(data)) => {
										view! {
											<For
												each={move || data.clone().machine_types}
												key={|state| state.id}
												let:child
											>
												<MachineTypeCard
													machine_type={child.clone()}
													is_selected={Signal::derive(move || {
														deployment_info
															.with(|info| {
																info.machine_type.is_some_and(|id| id == child.clone().id)
															})
													})}
													on_select={move |id: Uuid| {
														deployment_info.update(|info| info.machine_type = Some(id))
													}}
												/>
											</For>
										}
											.into_view()
									}
									_ => "Couldn't load resource".into_view(),
								}}
							</div>
						</Transition>
					</div>
				</div>
			</div>
		</div>
	}
}
