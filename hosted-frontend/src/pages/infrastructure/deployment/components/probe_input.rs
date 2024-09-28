use std::rc::Rc;

use ev::MouseEvent;
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
	/// On Delete
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_delete: Callback<MouseEvent>,
	/// Current Probe Value
	#[prop(optional, default = Signal::derive(move || None))]
	probe_value: Signal<Option<DeploymentProbe>>,
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
					{move || {
						view! {
							<InputDropdown
								placeholder={"Enter Probe Path".to_string()}
								value={probe_port}
								on_select={move |id: String| {
									probe_port.set(id.clone());
									on_select_port.call((id.clone(), probe_path.get().clone()))
								}}
								options={available_ports
									.get()
									.iter()
									.map(|x| InputDropdownOption {
										id: x.to_string(),
										label: x.to_string(),
										disabled: false,
									})
									.collect::<Vec<_>>()}
							/>
						}
					}}
				</div>

				<div class="flex-6 flex flex-col items-start justify-start">
					<Input
						r#type={InputType::Text}
						value={Signal::derive(move || {
							probe_value.get().map(|probe| { probe.path }).unwrap_or_default()
						})}
						on_input={Box::new(move |ev| {
							ev.prevent_default();
							on_input_path.call((probe_port.get().clone(), event_target_value(&ev)));
						})}
						class="w-full"
						placeholder={format!("Enter {} probe path", probe_type.as_str())}
					/>
				</div>

				<Show when={move || probe_value.get().is_some()}>
					// !probe_port.get().is_empty()
					<div class="flex-1 flex items-start justify-center">
						<Link
							style_variant={LinkStyleVariant::Plain}
							class="br-sm p-xs ml-md"
							should_submit=false
							on_click={Rc::new(move |ev| { on_delete.call(ev.clone()) })}
						>
							<Icon icon={IconType::Trash2} color={Color::Error} size={Size::Small} />
						</Link>
					</div>
				</Show>
			</div>
		</div>
	}
}
