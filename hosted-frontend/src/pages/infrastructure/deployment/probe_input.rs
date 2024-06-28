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
) -> impl IntoView {
	let outer_div_class = class.with(|cname| format!("flex full-width {}", cname));

	view! {
		<div class={outer_div_class}>
			<div class="flex-col-2 fc-fs-fs mb-auto mt-sm">
				<div class="fr-fs-fs">
					<label class="fc-fs-fs">{format!("{} Probe", probe_type.as_str())}</label>

					<ToolTipContainer icon_color={Color::White} tooltip_width=25.>
						<p class="txt-xxs">
							"Choose the Port on which your deployment is running and define the
							Path that needs to be checked by the probe."
							<a
								target="_blank"
								href="https://docs.patr.cloud/features/deployment/#step-3-create-a-deployment"
								rel="noopener noreferrer"
								class="txt-xxs txt-primary txt-underline txt-medium"
							>
								"Learn more"
							</a>
						</p>
					</ToolTipContainer>
				</div>

				<small class="txt-xxs txt-grey">

					{if probe_type == ProbeInputType::Liveness {
						"Restarts containers that are failing"
					} else {
						"Determines when the container is ready to accept requests"
					}}

				</small>
			</div>

			<div class="flex-col-10 fr-fs-fs">
				<div class="flex-col-5 pr-lg">
					<InputDropdown
						placeholder={"Enter Probe Path".to_string()}
						value={"6655".to_owned()}
						options={vec![
							InputDropdownOption {
								id: "1".to_string(),
								label: "6655".to_owned(),
								disabled: false,
							},
						]}
					/>

				</div>

				<div class="flex-col-6 fc-fs-fs">
					<Input
						r#type={InputType::Text}
						class="full-width"
						placeholder={format!("Enter {} probe path", probe_type.as_str())}
					/>
				</div>
			</div>
		</div>
	}
}
