use std::str::FromStr;

use ev::MouseEvent;
use models::api::workspace::deployment::*;

use super::{super::components::*, DeploymentInfoContext};
use crate::{prelude::*, queries::update_deployment_query};

/// Details tab for a deployment
#[component]
pub fn ManageDeploymentDetailsTab() -> impl IntoView {
	let deployment_info = expect_context::<DeploymentInfoContext>().0;
	let app_type = expect_context::<AppType>();

	let update_deployment_body = create_rw_signal(UpdateDeploymentRequest::new());

	let update_deployment_action = update_deployment_query();

	let on_click_submit = move |ev: MouseEvent| {
		ev.prevent_default();

		if let Some(deployment_info) = deployment_info.get() {
			update_deployment_action
				.dispatch((deployment_info.deployment.id, update_deployment_body.get()));
		}
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
								<label
									class="text-white text-sm mt-sm flex items-center justify-start"
									html_for="name"
								>
									"Name"
								</label>
							</div>

							<div class="flex-10 flex flex-col items-start justify-start">
								<Input
									r#type={InputType::Text}
									class="w-full"
									value={
										let deployment_info = info.clone();
										if let Some(name) = update_deployment_body.get().name {
											Signal::derive(move || name.clone())
										} else {
											Signal::derive(move || {
												deployment_info.deployment.name.clone()
											})
										}
									}
									on_input={Box::new(move |ev: web_sys::Event| {
										update_deployment_body
											.update(|body| body.name = Some(event_target_value(&ev)))
									})}
								/>
							</div>
						</div>

						<div class="flex w-full mb-md">
							<div class="flex-2 flex items-start justify-start">
								<label
									class="text-white text-sm mt-sm flex items-center justify-start"
									html_for="registry"
								>
									"Registry"
								</label>
							</div>

							<div class="flex-10">
								<Textbox disabled=true value={image_registry.into_view()} />
							</div>
						</div>

						<div class="flex w-full">
							<div class="flex-2 flex items-start justify-start">
								<label html_for="image-details">"Image Details"</label>
							</div>

							<div class="flex-7">
								<Textbox disabled=true value={image_name.into_view()} />
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

						{app_type
							.is_managed()
							.then(|| {
								view! {
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
								}
									.into_view()
							})}

						<PortInput
							ports_list={
								let ports = info.running_details.ports.clone();
								Signal::derive(move || ports.clone())
							}
							on_delete={move |port_number: String| {
								let port_number = StringifiedU16::from_str(port_number.as_str());
								if port_number.is_ok() {
									deployment_info
										.update(|info| {
											if let Some(info) = info {
												info.running_details.ports.remove(&port_number.unwrap());
											}
										});
									update_deployment_body
										.update(|body| {
											body.ports = deployment_info
												.get()
												.map(|info| info.running_details.ports);
										});
								}
							}}
							on_add={move |
								(port_number, port_type): (String, String)|
							{
								let port_number = StringifiedU16::from_str(port_number.as_str());
								let port_type = ExposedPortType::from_str(port_type.as_str());
								if port_number.is_ok() && port_type.is_ok() {
									deployment_info
										.update(|info| {
											if let Some(info) = info {
												info.running_details
													.ports
													.insert(port_number.unwrap(), port_type.unwrap());
											}
										});
									update_deployment_body
										.update(|body| {
											body.ports = deployment_info
												.get()
												.map(|info| info.running_details.ports);
										});
								}
							}}
							is_update_screen=true
						/>

						<EnvInput
							on_add={move |(name, value): (String, String)| {
								let env_val = EnvironmentVariableValue::String(value);
								if !name.is_empty() && env_val.value().is_some() {
									deployment_info
										.update(|info| {
											if let Some(info) = info {
												info.running_details
													.environment_variables
													.insert(name, env_val);
											}
										});
									update_deployment_body
										.update(|body| {
											body.environment_variables = deployment_info
												.get()
												.map(|info| info.running_details.environment_variables);
										});
								}
							}}
							on_delete={move |name: String| {
								deployment_info
									.update(|info| {
										if let Some(info) = info {
											info.running_details
												.environment_variables
												.remove(name.as_str());
										}
									});
								update_deployment_body
									.update(|body| {
										body.environment_variables = deployment_info
											.get()
											.map(|info| info.running_details.environment_variables);
									});
							}}
							envs_list={
								let envs = info.running_details.environment_variables.clone();
								Signal::derive(move || envs.clone())
							}
						/>

						<ProbeInput
							probe_type={ProbeInputType::Startup}
							probe_value={
								let startup_probe = info.running_details.startup_probe.clone();
								Signal::derive(move || startup_probe.clone())
							}
							available_ports={
								let ports = info.running_details.ports.clone();
								Signal::derive(move || {
									ports.keys().map(|port| port.value()).collect::<Vec<_>>()
								})
							}
							on_select_port={move |(port, path): (String, String)| {
								let probe_port = port.parse::<u16>();
								if let Ok(probe_port) = probe_port {
									deployment_info
										.update(|info| {
											if let Some(info) = info {
												info.running_details.startup_probe = Some(DeploymentProbe {
													port: probe_port,
													path: path.clone(),
												});
											}
										})
								}
								update_deployment_body
									.update(|body| {
										body.startup_probe = deployment_info
											.get()
											.map(|info| info.running_details.startup_probe)
											.flatten();
									});
							}}
							on_input_path={move |(port, path): (String, String)| {
								let probe_port = port.parse::<u16>();
								if let Ok(probe_port) = probe_port {
									deployment_info
										.update(|info| {
											if let Some(info) = info {
												info.running_details.startup_probe = Some(DeploymentProbe {
													port: probe_port,
													path: path.clone(),
												});
											}
										})
								}
								update_deployment_body
									.update(|body| {
										body.startup_probe = deployment_info
											.get()
											.map(|info| info.running_details.startup_probe)
											.flatten();
									});
							}}
							on_delete={move |_| {
								deployment_info
									.update(|info| {
										if let Some(info) = info {
											info.running_details.startup_probe = None;
										}
									});
								update_deployment_body
									.update(|body| {
										body.startup_probe = None;
									});
							}}
						/>

						<ProbeInput
							probe_type={ProbeInputType::Liveness}
							probe_value={
								let liveness_probe = info.running_details.liveness_probe.clone();
								Signal::derive(move || liveness_probe.clone())
							}
							available_ports={
								let ports = info.running_details.ports.clone();
								Signal::derive(move || {
									ports.keys().map(|port| port.value()).collect::<Vec<_>>()
								})
							}
							on_select_port={move |(port, path): (String, String)| {
								let probe_port = port.parse::<u16>();
								if let Ok(probe_port) = probe_port {
									deployment_info
										.update(|info| {
											if let Some(info) = info {
												info.running_details.liveness_probe = Some(DeploymentProbe {
													port: probe_port,
													path: path.clone(),
												});
											}
										})
								}
								update_deployment_body
									.update(|body| {
										body.liveness_probe = deployment_info
											.get()
											.map(|info| info.running_details.liveness_probe)
											.flatten();
									});
							}}
							on_input_path={move |(port, path): (String, String)| {
								let probe_port = port.parse::<u16>();
								if let Ok(probe_port) = probe_port {
									deployment_info
										.update(|info| {
											if let Some(info) = info {
												info.running_details.liveness_probe = Some(DeploymentProbe {
													port: probe_port,
													path: path.clone(),
												});
											}
										})
								}
								update_deployment_body
									.update(|body| {
										body.liveness_probe = deployment_info
											.get()
											.map(|info| info.running_details.liveness_probe)
											.flatten();
									});
							}}
							on_delete={move |_| {
								deployment_info
									.update(|info| {
										if let Some(info) = info {
											info.running_details.liveness_probe = None;
										}
									});
								update_deployment_body
									.update(|body| {
										body.liveness_probe = None;
									});
							}}
						/>
					</div>

					<div class="flex justify-end items-center gap-md w-full fit-wide-screen mx-auto mt-auto pb-xl px-xl">
						<button
							type="submit"
							class="flex items-center justify-center btn btn-primary"
							on:click={on_click_submit}
						>
							"UPDATE"
						</button>
					</div>
				}
			}
			.into_view(),
			None => view! { "error occurred" }.into_view(),
		}
	}
}
