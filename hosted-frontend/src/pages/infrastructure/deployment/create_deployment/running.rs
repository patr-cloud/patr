use std::str::FromStr;

use ev::MouseEvent;
use models::api::workspace::deployment::{
	DeploymentVolume,
	EnvironmentVariableValue,
	ExposedPortType,
};

use super::RunnerPageError;
use crate::{
	pages::{
		ConfigMountInput,
		DeploymentInfo,
		EnvInput,
		PortInput,
		ProbeInput,
		ProbeInputType,
		VolumeInput,
	},
	prelude::*,
};

#[component]
pub fn RunningDetails(
	/// The Errors For This Page
	#[prop(into)]
	errors: MaybeSignal<RunnerPageError>,
) -> impl IntoView {
	let deployment_info = expect_context::<RwSignal<DeploymentInfo>>();

	view! {
		<div class="fc-fs-fs full-width fit-wide-screen px-xl mt-xl">
			<h4 class="txt-white txt-lg pb-md txt-white">"Deployment Details"</h4>

			<div class="fc-fs-fs gap-xl full-width full-height txt-white">
				<PortInput
					on_add=move |(_, port_number, port_type): (MouseEvent, String, String)| {
						let port_number = StringifiedU16::from_str(port_number.as_str());
						let port_type = ExposedPortType::from_str(port_type.as_str());
						if port_number.is_ok() && port_type.is_ok() {
							deployment_info.update(|info| {
								info.ports.insert(port_number.unwrap(), port_type.unwrap());
							});
						}
					}
					on_delete=move |(_, port_number): (MouseEvent, String)| {
						let port_number = StringifiedU16::from_str(port_number.as_str());
						if port_number.is_ok() {
							deployment_info.update(|info| {
								info.ports.remove(&port_number.unwrap());
							});
						}

					}
					error={Signal::derive(move || errors.get().ports)}
					is_update_screen={false}
					ports_list={Signal::derive(move || deployment_info.get().ports)}
				/>

				<EnvInput
					on_add=move |(_, name, value): (MouseEvent, String, String)| {
						let env_value = EnvironmentVariableValue::String(value);

						if !name.is_empty() && env_value.value().is_some() {
							deployment_info.update(|info| {
								info.environment_variables.insert(name, env_value);
							});
						}
					}
					on_delete=move |(_, name): (MouseEvent, String)| {
						deployment_info.update(|info| {
							info.environment_variables.remove(name.as_str());
						});
					}
					envs_list={Signal::derive(move || deployment_info.get().environment_variables)}
				/>

				<ConfigMountInput
					mount_points={vec!["/x/y/path".to_owned()]}
				/>

				<VolumeInput
					on_add=move |(_, path, size): (MouseEvent, String, String)| {
						let vol_size = size.parse::<u16>();
						if !path.is_empty() && vol_size.is_ok() {
							let deployment_vol = DeploymentVolume {
								path,
								// Already checking if it's Ok above
								// Hence, safe to unwrap
								size: vol_size.unwrap(),
							};

							deployment_info.update(|info| {
								info.volumes.insert(Uuid::new_v4(), deployment_vol);
							});
						}
					}
					on_delete=move |(_, id): (MouseEvent, Uuid)| {
						deployment_info.update(|info| {
							info.volumes.remove(&id);
						});
					}
					volumes_list={Signal::derive(move || deployment_info.get().volumes)}
				/>

				<ProbeInput
					available_ports={Signal::derive(
						move || deployment_info.get()
							.ports
							.iter()
							.map(|(port, _)| port.value())
							.collect::<Vec<_>>()
					)}
					probe_type={ProbeInputType::Startup}
					on_select_port={move |(port, path): (String, String)| {
						let probe_port = port.parse::<u16>();
						if let Ok(probe_port) = probe_port {
							deployment_info.update(|info| {
								info.startup_probe = Some((probe_port, path))
							});
						}
					}}
					on_input_path={move |(port, path): (String, String)| {
						let probe_port = port.parse::<u16>();
						if let Ok(probe_port) = probe_port {
							deployment_info.update(|info| {
								info.startup_probe = Some((probe_port, path.clone()))
							});
						}
					}}
				/>

				<ProbeInput
					available_ports={Signal::derive(
						move || deployment_info.get()
							.ports
							.iter()
							.map(|(port, _)| port.value())
							.collect::<Vec<_>>()
					)}
					probe_type={ProbeInputType::Liveness}
					on_select_port={move |(port, path): (String, String)| {
						let probe_port = port.parse::<u16>();
						if let Ok(probe_port) = probe_port {
							deployment_info.update(|info| {
								info.liveness_probe = Some((probe_port, path))
							});
						}
					}}
					on_input_path={move |(port, path): (String, String)| {
						let probe_port = port.parse::<u16>();
						if let Ok(probe_port) = probe_port {
							deployment_info.update(|info| {
								info.liveness_probe = Some((probe_port, path.clone()))
							});
						}
					}}
				/>
			</div>
		</div>
	}
}
