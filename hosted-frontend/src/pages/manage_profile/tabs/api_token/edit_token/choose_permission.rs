use convert_case::{Case, Casing};
use models::rbac::ResourceType;
use strum::VariantNames;

use crate::prelude::*;

#[component]
pub fn ChoosePermission(
	/// Additional class names to apply to the, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// How many columns are there in the grid, can be "auto" or a number as a
	/// String
	#[prop(into, optional, default = "auto".to_string().into())]
	grid_columns: MaybeSignal<String>,
) -> impl IntoView {
	let div_class = class.with(|cname| {
		format!(
			"gap-sm grid grid-col-{} full-width {}",
			grid_columns.get(),
			cname
		)
	});

	let input_resource_type = create_rw_signal("".to_string());
	let input_resources = create_rw_signal::<Vec<String>>(vec![]);
	let input_permissions = create_rw_signal("".to_string());

	let resource_type_values = ResourceType::VARIANTS
		.iter()
		.map(|v| InputDropdownOption {
			label: v.to_case(Case::Title).to_string(),
			disabled: false,
		})
		.collect::<Vec<InputDropdownOption>>();

	let show_resource_type =
		create_memo(
			move |_| match input_resource_type.get().to_case(Case::Camel).as_str() {
				"dnsRecord" => false,
				"workspace" => false,
				_ => true,
			},
		);

	create_effect(move |_| {
		logging::log!("resource_type: {}", input_resource_type.get());
		logging::log!("show_resource_type: {}", show_resource_type.get());
	});

	view! {
		<div class="fr-fs-fs full-width">
			<div class={div_class}>
				<div class="fc-fs-fs">
					<InputDropdown
						placeholder="Select Resource Type".to_string()
						options={resource_type_values}
						value={input_resource_type}
					/>
				</div>

				<Show when={move || show_resource_type.get()}>
					<div class="fc-fs-fs">
						<InputDropdown
							placeholder={format!("All/Specific {}", input_resource_type.with(|resource|
								if resource.is_empty() {"Resource".to_string()} else {resource.to_owned()}
							))}
							options={vec![
								InputDropdownOption {
									label: format!("All {}", input_resource_type.get()),
									disabled: false,
								},
								InputDropdownOption {
									label: format!("Specific {}", input_resource_type.get()),
									disabled: false,
								},
								InputDropdownOption {
									label: format!("All {} Except", input_resource_type.get() ).to_string(),
									disabled: false,
								},
							]}
						/>
					</div>
				</Show>

				<Show when={move || show_resource_type.get()}>
					<CheckboxDropdown
						on_select={|(_, label)| {
							logging::log!("Selected: {}", label);
						}}
						placeholder={format!("Select {}", input_resource_type.with(|resource|
							if resource.is_empty() {"Resources".to_string()} else {resource.to_owned()}
						))}
						options={vec![]}
					/>
				</Show>

				<div>
					<CheckboxDropdown
						placeholder="Select Permissions".to_string()
						options={vec![
							InputDropdownOption {
								label: "Info".to_string(),
								disabled: false,
							},
							InputDropdownOption {
								label: "Write".to_string(),
								disabled: false,
							},
							InputDropdownOption {
								label: "Edit".to_string(),
								disabled: false,
							},
							InputDropdownOption {
								label: "Delete".to_string(),
								disabled: false,
							},
						]}
					/>
				</div>
			</div>

			<div class="fr-ct-ct pl-md">
				<Link
					r#type=Variant::Button
					class="br-sm p-xs"
					style_variant=LinkStyleVariant::Contained
				>
					<Icon icon={IconType::Plus} color=Color::Secondary />
				</Link>
			</div>
		</div>
	}
}
