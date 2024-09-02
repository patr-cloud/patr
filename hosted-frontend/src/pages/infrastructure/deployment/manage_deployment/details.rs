use std::str::FromStr;

use ev::MouseEvent;
use models::api::workspace::deployment::*;

use super::{super::components::*, DeploymentInfoContext};
use crate::prelude::*;

/// Details tab for a deployment
#[component]
pub fn ManageDeploymentDetailsTab() -> impl IntoView {
	let deployment_info = expect_context::<DeploymentInfoContext>().0;
	let (state, _) = AuthState::load();

	let navigate = leptos_router::use_navigate();
	let on_click_submit = move |_: MouseEvent| {
		let navigate = navigate.clone();
		spawn_local(async move {
			if let Some(deployment_info) = deployment_info.get() {
				let resp = update_deployment(
					state.get().get_last_used_workspace_id(),
					state.get().get_access_token(),
					Some(deployment_info.deployment.id.to_string()),
					Some(deployment_info.deployment.name.clone()),
					Some(deployment_info.deployment.machine_type.to_string()),
					Some(deployment_info.running_details.deploy_on_push),
					Some(deployment_info.running_details.min_horizontal_scale),
					Some(deployment_info.running_details.max_horizontal_scale),
					Some(deployment_info.running_details.ports),
					Some(deployment_info.running_details.environment_variables),
					deployment_info.running_details.startup_probe,
					deployment_info.running_details.liveness_probe,
					Some(deployment_info.running_details.config_mounts),
					Some(deployment_info.running_details.volumes),
				)
				.await;

				if resp.is_ok() {
					navigate("/deployment", Default::default());
				}
			}
		})
	};

	move || {
		match deployment_info.get() {
			Some(info) => {
				let (image_registry, image_name) = match &info.clone().deployment.registry {
					DeploymentRegistry::PatrRegistry {
						registry: _,
						repository_id,
					} => ("Patr Registry".to_string(), repository_id.to_string()),
					DeploymentRegistry::ExternalRegistry {
						registry,
						image_name,
					} => (registry.to_owned(), image_name.to_owned()),
				};

				view! {
					<div class="flex flex-col items-start justify-start w-full px-xl pb-xl mt-xl text-white gap-md fit-wide-screen mx-auto">
						<div class="flex w-full">
							<div class="flex-2 flex items-start justify-start">
								<label class="text-white text-sm mt-sm flex items-center justify-start" html_for="name">
									"Name"
								</label>
							</div>

							<div class="flex-10 flex flex-col items-start justify-start">
								<Input
									r#type={InputType::Text}
									class="w-full"
									value={
										let deployment_info = info.clone();
										Signal::derive(move || deployment_info.clone().deployment.clone().name.clone())
									}
								/>
							</div>
						</div>

						<div class="flex w-full mb-md">
							<div class="flex-2 flex items-start justify-start">
								<label class="text-white text-sm mt-sm flex items-center justify-start" html_for="registry">
									"Registry"
								</label>
							</div>

							<div class="flex-10">
								<Textbox disabled=true value={image_registry.into_view()}/>
							</div>
						</div>

						<div class="flex w-full">
							<div class="flex-2 flex items-start justify-start">
								<label html_for="image-details">"Image Details"</label>
							</div>

							<div class="flex-7">
								<Textbox
									disabled=true
									value={image_name.into_view()}
								/>
							</div>
							<div class="flex-3 pl-md">
								<Textbox
									disabled=true
									value={
										let deployment_tag = info.deployment.image_tag.clone();
										(move || deployment_tag.clone()).into_view()
									}
								/>
							</div>
						</div>

						<div class="flex w-full">
							<div class="flex-2 flex items-start justify-start">
								<label html_for="image-details">"Runner"</label>
							</div>
							<div class="flex-10 flex flex-col items-start justify-start">
								<Textbox
									value={
										let runner_id = info.deployment.runner;
										(move || runner_id.to_string()).into_view()
									}
									disabled=true
								/>
							</div>
						</div>

						<PortInput
							ports_list={
								let ports = info.running_details.ports.clone();
								Signal::derive(move || ports.clone())
							}
							on_delete=move |(_, port_number): (MouseEvent, String)| {
								let port_number = StringifiedU16::from_str(port_number.as_str());
								if port_number.is_ok() {
									deployment_info.update(|info| {
										if let Some(info) = info {
											info.running_details.ports.remove(&port_number.unwrap());
										}
									});
								}
							}
							on_add=move |(_, port_number, port_type): (MouseEvent, String, String)| {
								let port_number = StringifiedU16::from_str(port_number.as_str());
								let port_type = ExposedPortType::from_str(port_type.as_str());

								if port_number.is_ok() && port_type.is_ok() {
									deployment_info.update(|info| {
										if let Some(info) = info {
											info.running_details.ports.insert(port_number.unwrap(), port_type.unwrap());
										}
									});
								}
							}
							is_update_screen=true
						/>

						<EnvInput
							on_add=move |(_, name, value): (MouseEvent, String, String)| {
								let env_val = EnvironmentVariableValue::String(value);

								if !name.is_empty() && env_val.value().is_some() {
									deployment_info.update(|info| {
										if let Some(info) = info {
											info.running_details.environment_variables.insert(name, env_val);
										}
									});
								}
							}
							on_delete=move |(_, name): (MouseEvent, String)| {
								deployment_info.update(|info| {
									if let Some(info) = info {
										info.running_details.environment_variables.remove(name.as_str());
									}
								});
							}
							envs_list={
								let envs = info.running_details.environment_variables.clone();
								Signal::derive(move || envs.clone())
							}
						/>

						<ConfigMountInput mount_points={vec!["/x/y/path".to_owned()]}/>

						<ProbeInput
							probe_value={
								let startup_probe = info.running_details.startup_probe.clone();
								Signal::derive(move || startup_probe.clone())
							}
							available_ports={
								let ports = info.running_details.ports.clone();

								Signal::derive(
									move || ports.keys().map(|port| port.value())
										.collect::<Vec<_>>()
								)
							}
							probe_type={ProbeInputType::Startup}
						/>

						<ProbeInput
							probe_value={
								let liveness_probe = info.running_details.liveness_probe.clone();
								Signal::derive(move || liveness_probe.clone())
							}
							available_ports={
								let ports = info.running_details.ports.clone();
								Signal::derive(
									move || ports.keys().map(|port| port.value())
										.collect::<Vec<_>>()
								)
							}
							probe_type={ProbeInputType::Liveness}
						/>
					</div>

					<div class="flex justify-end items-center gap-md w-full fit-wide-screen mx-auto mt-auto pb-xl px-xl">
						<button
							type="submit"
							class="flex items-center justify-center btn btn-primary"
							on:click={on_click_submit.clone()}
						>
							"UPDATE"
						</button>
					</div>
				}
			}
			.into_view(),
			None => view! {"error occurred"}.into_view(),
		}
	}
}
