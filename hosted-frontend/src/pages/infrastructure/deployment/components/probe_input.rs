use models::api::workspace::deployment::DeploymentProbe;

use crate::prelude::*;

/// The type of the probe input.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum ProbeInputType {
	/// The probe input is for the startup probe.
	Startup,
	/// The probe input is for the liveness probe.
	Liveness,
}

impl ProbeInputType {
	/// Returns the CSS name of the size.
	pub const fn as_str(&self) -> &'static str {
		match self {
			Self::Startup => "Startup",
			Self::Liveness => "Liveness",
		}
	}
}

#[component]
pub fn ProbeInput(
	/// Additional class names to apply to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The type of the input
	probe_type: ProbeInputType,
	/// List of all available Ports
	#[prop(into, optional, default = vec![].into())]
	available_ports: MaybeSignal<Vec<u16>>,
	/// On Select Port
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_select_port: Callback<(String, String)>,
	/// On Enter Path
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_input_path: Callback<(String, String)>,
	/// Current Probe Value
	#[prop(into, optional, default = None.into())]
	probe_value: MaybeSignal<Option<DeploymentProbe>>,
) -> impl IntoView {
	let outer_div_class = class.with(|cname| format!("flex w-full {}", cname));

	let probe_port = create_rw_signal(
		if let Some(probe_value) = probe_value.get_untracked() {
			probe_value.port.to_string()
		} else {
			"".to_owned()
		},
	);
	let probe_path = create_rw_signal(
		if let Some(probe_value) = probe_value.get_untracked() {
			probe_value.path
		} else {
			"".to_owned()
		},
	);

	view! {
		<div class={outer_div_class}>
			<div class="flex-2 flex flex-col items-start justify-start mb-auto mt-sm">
				<div class="flex items-start justify-start">
					<label class="flex flex-col items-start justify-start">
						{format!("{} Probe", probe_type.as_str())}
					</label>
				</div>

				<small class="text-xxs text-grey">

					{if probe_type == ProbeInputType::Liveness {
						"Restarts containers that are failing"
					} else {
						"Determines when the container is ready to accept requests"
					}}

				</small>
			</div>

			<div class="flex-10 flex items-start justify-start">
				<div class="flex-5 pr-lg">
					{
						move || view! {
							<InputDropdown
								placeholder={"Enter Probe Path".to_string()}
								value={probe_port}
								// on_select={move |id: String| {
								// 	on_select_port.call((id.clone(), probe_path.get().clone()));
								// }}
								on_select={move |id: String| {
									probe_port.set(id.clone());
									on_select_port.call((id.clone(), probe_path.get().clone()))
								}}
								options={
									available_ports.get()
										.iter()
										.map(|x| InputDropdownOption {
											id: x.to_string(),
											label: x.to_string(),
											disabled: false,
										})
										.collect::<Vec<_>>()
								}
							/>
						}
					}
				</div>

				<div class="flex-6 flex flex-col items-start justify-start">
					<Input
						r#type={InputType::Text}
						value={Signal::derive(move || probe_path.get())}
						on_input={Box::new(move |ev| {
							ev.prevent_default();
							probe_path.set(event_target_value(&ev));
							on_input_path.call((probe_port.get().clone(), event_target_value(&ev)));
						})}
						class="w-full"
						placeholder={format!("Enter {} probe path", probe_type.as_str())}
					/>
				</div>
			</div>
		</div>
	}
}
