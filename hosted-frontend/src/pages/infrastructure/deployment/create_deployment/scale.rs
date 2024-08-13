use codee::string::FromToStringCodec;
use leptos_use::use_cookie;

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
		<div class="fc-fs-fs w-full px-xl mt-xl text-white text-sm fit-wide-screen mx-auto gap-md">
			<h4 class="text-white text-lg pb-md">"Scale Your Servers"</h4>

			<div class="flex w-full">
				<div class="flex-2 my-auto pr-md">
					<span class="text-sm">"Choose Horizontal Scale"</span>
				</div>

				<div class="flex-10 flex flex-col justify-start items-center bg-secondary-light p-xl br-sm">
					<p class="w-full tracking-[1px] mb-lg text-xxs">
						"Choose the minimum and maximum number of instances for your deployment "
					</p>

					<div class="w-full flex items-center justify-center">
						<div class="flex-2 flex flex-col items-center justify-center">
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

						<div class="flex-8 mt-xl px-xl flex flex-col items-center justify-start">
							<DoubleInputSlider
								min={min_horizontal}
								max={max_horizontal}
								min_limit={1}
								max_limit={10}
								class="w-full"
							/>

							<p class="text-warning text-xxs">
								"Any excess volumes will be removed if the number of instances is reduced."
							</p>
						</div>

						<div class="flex-2 flex flex-col justify-center items-center">
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
								{
									move || match machine_list.get() {
										Some(Ok(data)) => view! {
											<For
												each={move || data.clone().machine_types}
												key={|state| state.id}
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
