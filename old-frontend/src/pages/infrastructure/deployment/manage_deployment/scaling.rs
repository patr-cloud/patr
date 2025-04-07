use std::{f32::consts::E, rc::Rc};

use ev::MouseEvent;
use models::api::workspace::deployment::*;

use super::DeploymentInfoContext;
use crate::{
	pages::infrastructure::deployment::components::MachineTypeCard,
	prelude::*,
	queries::{list_machines_query, update_deployment_query},
};

#[component]
fn UpdateScale(
	/// Deployment Info
	#[prop(into)]
	deployment_info: GetDeploymentInfoResponse,
	/// Update Deployment Info Body
	update_deployment_body: RwSignal<UpdateDeploymentRequest>,
) -> impl IntoView {
	let store_deployment = store_value(deployment_info.clone());
	let min_horizontal_value =
		create_rw_signal(deployment_info.running_details.min_horizontal_scale);
	let max_horizontal_value =
		create_rw_signal(deployment_info.running_details.max_horizontal_scale);

	let deployment_info_context = expect_context::<DeploymentInfoContext>().0;

	view! {
		<div class="w-full flex items-center justify-center">
			<div class="flex-2 flex flex-col items-center justify-center">
				<label html_for="minHorizontalScale">"Minimum Scale"</label>

				<NumberPicker
					value={min_horizontal_value}
					style_variant={SecondaryColorVariant::Medium}
					on_change={move |_| update_deployment_body.update(|body| {
						body.min_horizontal_scale = Some(min_horizontal_value.get());
					})}
				/>
			</div>

			<div class="flex-8 mt-xl px-xl flex flex-col items-center justify-start">
				<p class="text-warning text-xxs">
					"Any excess volumes will be removed if the number of instances is reduced."
				</p>
			</div>

			<div class="flex-2 flex flex-col items-center justify-center">
				<label html_for="maxHorizontalScale">"Maximum Scale"</label>

				<NumberPicker
					value={max_horizontal_value}
					style_variant={SecondaryColorVariant::Medium}
					on_change={move |_| update_deployment_body.update(|body| {
						body.max_horizontal_scale = Some(max_horizontal_value.get());
					})}
				/>
			</div>
		</div>
	}
}

/// Update Machine Type of Deployment
#[component]
fn UpdateMachineType(
	/// Deployment Info
	#[prop(into)]
	deployment_info: GetDeploymentInfoResponse,
	/// Update Deployment Info Body
	update_deployment_body: RwSignal<UpdateDeploymentRequest>,
) -> impl IntoView {
	let machine_list = list_machines_query();

	let store_deployment = store_value(deployment_info.clone());
	let deployment_info = expect_context::<DeploymentInfoContext>().0;

	view! {
		<Transition>
			{move || match machine_list.get() {
				Some(
					Ok(ListAllDeploymentMachineTypeResponse { machine_types }),
				) => {
					view! {
						<For
							each={move || machine_types.clone()}
							key={|state| state.id.clone()}
							let:machine_type
						>
							<MachineTypeCard
								machine_type={machine_type.clone()}
								is_selected={Signal::derive(
									move || store_deployment.with_value(|deployment| {
										deployment.deployment.machine_type == machine_type.id.clone()
									})
								)}
								on_select={move |id: Uuid| {
									update_deployment_body.update(|body| {
										body.machine_type = Some(id.clone());
									});
									deployment_info.update(|info| {
										if let Some(info) = info {
											info.deployment.data.machine_type = id.clone();
										}
									});
								}}
							/>
						</For>
					}
						.into_view()
				}
				_ => "Loading...".into_view(),
			}}
		</Transition>
	}
}

/// The Scaling page of the deployment management page.
#[component]
pub fn ManageDeploymentScaling() -> impl IntoView {
	let app_type = expect_context::<AppType>();
	let deployment_info = expect_context::<DeploymentInfoContext>().0;

	let update_deployment_body = create_rw_signal(UpdateDeploymentRequest::new());

	let update_deployment_action = update_deployment_query();

	let on_submit = move |ev: &MouseEvent| {
		ev.prevent_default();
		if let Some(deployment_info) = deployment_info.get() {
			update_deployment_action
				.dispatch((deployment_info.deployment.id, update_deployment_body.get()));
		}
	};

	view! {
		<div class="flex flex-col items-start justify-start w-full px-xl mt-xl
			text-white text-sm fit-wide-screen mx-auto gap-md"
		>
			<div class="flex w-full">
				<div class="flex-2 my-auto pr-md">
					<span class="text-sm">"Choose Horizontal Scale"</span>
				</div>

				<div class="flex-10 fc-fs-ct flex flex-col items-center justify-start bg-secondary-light p-xl br-sm">
					<p class="w-full tracking-[1px] mb-lg text-xxs">
						"Choose the minimum and maximum number of instances for your deployment "
					</p>

					{
						move || match deployment_info.get() {
							Some(deployment_info) => view! {
								<UpdateScale
									deployment_info={deployment_info}
									update_deployment_body={update_deployment_body}
								/>
							}.into_view(),
							None => view! {
								<div>"Loading..."</div>
							}.into_view()
						}
					}
				</div>
			</div>

			<div class="flex w-full">
				<div class="flex-2 my-auto pr-md">
					<span class="text-sm">"Manage Resource Allocation"</span>
				</div>

				<div class="flex-10 flex items-center justify-start overflow-auto">
					<div class="w-full p-xl br-sm bg-secondary-light
					flex flex-col items-start justify-start overflow-auto">
						<p class="tracking-[1px] mb-lg text-xxs">
							"Specify the resources to be allocated to your container"
						</p>

						<div class="flex items-center justify-start overflow-x-auto py-xxs gap-md">
							{
								move || match deployment_info.get() {
									Some(deployment_info) => view! {
										<UpdateMachineType
											deployment_info={deployment_info}
											update_deployment_body={update_deployment_body}
										/>
									}.into_view(),
									None => view! {
										<div>"Loading..."</div>
									}.into_view()
								}
							}
						</div>
					</div>
				</div>
			</div>

			{app_type
				.is_managed()
				.then(|| {
					view! {
						<div class="flex w-full">
							<div class="flex-2 my-auto pr-md">
								<span class="text-sm">"Estimated Cost"</span>
							</div>

							<div class="flex-10 flex flex-col items-start justify-start overflow-auto">
								<div class="flex items-center justify-start">
									<span class="text-xl text-success text-thin">
										"$5" <small class="text-grey text-lg">"/month"</small>
									</span>
								</div>

								<p class="text-grey">
									"This deployment is eligible for "
									<strong class="text-medium text-sm">"Free"</strong> "plan"
									"since it's your first deployment and" <br />
									"you have selected the base machine type with only one instance."
								</p>
							</div>
						</div>
					}
						.into_view()
				})}
		</div>

		<div class="flex justify-end items-center gap-md w-full fit-wide-screen mx-auto mt-auto pb-xl px-xl">
			<Link
				style_variant={LinkStyleVariant::Contained}
				r#type={Variant::Button}
				should_submit=true
				on_click={Rc::new(on_submit)}
			>
				"UPDATE"
			</Link>
		</div>
	}
}
