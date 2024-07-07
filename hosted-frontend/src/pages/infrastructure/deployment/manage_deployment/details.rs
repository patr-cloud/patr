use std::{collections::BTreeMap, str::FromStr};

use ev::MouseEvent;
use models::api::workspace::deployment::*;
use utils::FromToStringCodec;

use crate::{pages::*, prelude::*};

#[component]
pub fn ManageDeploymentDetailsTab() -> impl IntoView {
	let deployment_info = expect_context::<DeploymentInfoContext>().0;
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let navigate = leptos_router::use_navigate();
	let on_click_submit = move |_: MouseEvent| {
		let navigate = navigate.clone();
		spawn_local(async move {
			match deployment_info.get() {
				Some(deployment_info) => {
					let resp = update_deployment(
						current_workspace_id.get(),
						access_token.get(),
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
				None => {}
			}
		})
	};

	move || match deployment_info.get() {
		Some(info) => {
			let (image_registry, image_name) = match &info.clone().deployment.registry {
				DeploymentRegistry::PatrRegistry {
					registry,
					repository_id,
				} => ("Patr Registry".to_string(), repository_id.to_string()),
				DeploymentRegistry::ExternalRegistry {
					registry,
					image_name,
				} => (registry.to_owned(), image_name.to_owned()),
			};

			view! {
				<div class="fc-fs-fs full-width px-xl pb-xl mt-xl txt-white gap-md fit-wide-screen mx-auto">
					<div class="flex full-width">
						<div class="flex-col-2 fr-fs-fs">
							<label class="txt-white txt-sm mt-sm fr-fs-ct" html_for="name">
								"Name"
							</label>
						</div>

						<div class="flex-col-10 fc-fs-fs">
							<Input
								r#type={InputType::Text}
								class="full-width"
								value={
									let deployment_info = info.clone();
									Signal::derive(move || deployment_info.clone().deployment.clone().name.clone())
								}
							/>
						</div>
					</div>

					<div class="flex full-width mb-md">
						<div class="flex-col-2 fr-fs-fs">
							<label class="txt-white txt-sm mt-sm fr-fs-ct" html_for="registry">
								"Registry"
							</label>
						</div>

						<div class="flex-col-10">
							<Textbox disabled=true value={image_registry.into_view()}/>
						</div>
					</div>

					<div class="flex full-width">
						<div class="flex-col-2 fr-fs-fs">
							<label html_for="image-details">"Image Details"</label>
						</div>

						<div class="flex-col-7">
							<Textbox
								disabled=true
								value={image_name.into_view()}
							/>
						</div>
						<div class="flex-col-3 pl-md">
							<Textbox
								disabled=true
								value={
									let deployment_tag = info.deployment.image_tag.clone();
									(move || deployment_tag.clone()).into_view()
								}
							/>
						</div>
					</div>

					<div class="flex full-width">
						<div class="flex-col-2 fr-fs-fs">
							<label html_for="image-details">"Runner"</label>
						</div>
						<div class="flex-col-10 fc-fs-fs">
							<Textbox
								value={
									let runner_id = info.deployment.runner.clone();
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
								move || ports
									.iter()
									.map(|(port, _)| port.value())
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
								move || ports
									.iter()
									.map(|(port, _)| port.value())
									.collect::<Vec<_>>()
							)
						}
						probe_type={ProbeInputType::Liveness}
					/>
				</div>

				<div class="fr-fe-ct gap-md full-width fit-wide-screen mx-auto mt-auto pb-xl px-xl">
					<button
						type="submit"
						class="fr-ct-ct btn btn-primary"
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
