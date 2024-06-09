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

	let resource_type_values = ResourceType::VARIANTS
		.iter()
		.map(|v| InputDropdownOption {
			label: v.to_case(Case::Title).to_string(),
			disabled: false,
		})
		.collect::<Vec<InputDropdownOption>>();

	let resource_type_input = create_rw_signal("".to_string());
	let show_resource_type = create_memo(move |_| match resource_type_input.get().as_str() {
		"dnsRecord" => true,
		"workspace" => true,
		_ => true,
	});

	view! {
		<div class="fr-fs-fs full-width">
			<div class={div_class}>
				<div class="fc-fs-fs">
					<InputDropdown
						placeholder="Select Resource Type".to_string()
						options={resource_type_values}
						value={resource_type_input}
					/>
				</div>

				<div>
					<Show when={move || show_resource_type.get()}>
						<InputDropdown
							placeholder="Select Workspace".to_string()
							options={vec![
								InputDropdownOption {
									label: "All Domains".to_string(),
									disabled: false,
								},
								InputDropdownOption {
									label: "Specific Domains".to_string(),
									disabled: false,
								},
								InputDropdownOption {
									label: "All Domains Except".to_string(),
									disabled: false,
								},
							]}
						/>
					</Show>
				</div>

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
