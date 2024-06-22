use std::{str::FromStr, string::ParseError};

use convert_case::{Case, Casing};
use leptos_use::utils::FromToStringCodec;
use models::rbac::ResourceType;
use strum::{EnumString, VariantNames};

use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, VariantNames)]
#[strum(serialize_all = "camelCase")]
pub enum ApplyToOptions {
	AllResource,
	Specific,
	Except,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseApplyToOptions;

impl FromStr for ApplyToOptions {
	type Err = ParseApplyToOptions;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s.contains("Specific") {
			return Ok(Self::Specific);
		} else if s.contains("Except") {
			return Ok(Self::Except);
		} else if s.contains("All") {
			return Ok(Self::AllResource);
		} else {
			return Err(ParseApplyToOptions);
		}
	}
}

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

	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let resource = create_resource(
		move || access_token.get(),
		move |value| async move { list_user_workspace(value).await },
	);

	let input_resource_type = create_rw_signal("".to_string());
	let input_apply_to = create_rw_signal("".to_string());
	let input_resources = create_rw_signal::<Vec<String>>(vec![]);
	let input_permissions = create_rw_signal("".to_string());

	let resource_type_values = ResourceType::VARIANTS
		.iter()
		.map(|v| InputDropdownOption {
			label: v.to_case(Case::Title).to_string(),
			disabled: false,
		})
		.collect::<Vec<InputDropdownOption>>();

	let show_resource_type = create_memo(move |_| {
		match ResourceType::from_str(input_resource_type.get().to_case(Case::Camel).as_str()) {
			Ok(ResourceType::DnsRecord) => false,
			Ok(ResourceType::Workspace) => true,
			_ => true,
		}
	});

	let show_resources = move || {
		show_resource_type.get() &&
			ApplyToOptions::from_str(input_apply_to.get().as_str())
				.is_ok_and(|option| !matches!(option, ApplyToOptions::AllResource))
	};

	create_effect(move |_| {
		logging::log!("resource_type: {}", input_resource_type.get());
		logging::log!("show_resource_type: {}", show_resource_type.get());
		logging::log!(
			"input_apply_to: {:#?}",
			ApplyToOptions::from_str(input_apply_to.get().as_str())
		);
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
							value={input_apply_to}
							options={vec![
								InputDropdownOption {
									label: format!("All {}s", input_resource_type.get()),
									disabled: false,
								},
								InputDropdownOption {
									label: format!("Specific {}", input_resource_type.get()),
									disabled: false,
								},
								InputDropdownOption {
									label: format!("All {}s Except", input_resource_type.get() ).to_string(),
									disabled: false,
								},
							]}
						/>
					</div>
				</Show>

				<Show when={show_resources}>
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
