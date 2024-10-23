use std::{
	collections::{BTreeMap, BTreeSet},
	str::FromStr,
};

use convert_case::{Case, Casing};
use ev::MouseEvent;
use models::rbac::{ResourcePermissionType, ResourceType, WorkspacePermission};
use strum::VariantNames;

use super::{super::utils::ApiTokenPermissions, PermissionsDropdown, ResourceDropdownOptions};
use crate::prelude::*;

/// Enum that specifies whether to apply the permission to all resources, a
/// specific set of resources, or all resources except a specific set of
/// resources.15
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, VariantNames)]
#[strum(serialize_all = "camelCase")]
pub enum ApplyToOptions {
	/// Apply the permissions to all resources.
	AllResource,
	/// Apply the permissions to a specific set of resources. Specified in a
	/// seperate InputDropdown.
	Specific,
	/// Apply the permissions to all resources except a specific set of
	/// resources. Specified in a seperate InputDropdown.
	Except,
}

/// A struct that holds the parsed [Permission][perm] Info.
/// Splits the name field in [Permission] Struct into `name` and `resource_type`
///
/// TODO: Do all this in permissions only, i.e. have a from_str implementation
/// on permission that will do the splitting and eliminate the need for this
/// struct
///
/// TODO: Figure out a way to point this to the actual thing
/// [perm]: struct@models::rbac::Permission
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPermission {
	/// The name of the permission
	pub name: String,
	/// The Resource Type
	pub resource_type: ResourceType,
	/// Details of the permission
	pub details: String,
}

/// An Error Struct that is thrown when the [`ApplyToOptions`] fails to parse.
#[derive(Debug, PartialEq, Eq)]
pub struct ParseApplyToOptionsError;

impl FromStr for ApplyToOptions {
	type Err = ParseApplyToOptionsError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s.contains("Specific") {
			Ok(Self::Specific)
		} else if s.contains("Except") {
			Ok(Self::Except)
		} else if s.contains("All") {
			Ok(Self::AllResource)
		} else {
			Err(ParseApplyToOptionsError)
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
	/// The workspace id of the workspace
	#[prop(into)]
	workspace_id: MaybeSignal<Uuid>,
) -> impl IntoView {
	let (state, _) = AuthState::load();

	let div_class = class.with(|cname| {
		format!(
			"gap-sm grid grid-col-{} w-full {}",
			grid_columns.get(),
			cname
		)
	});

	let api_token_permissions = expect_context::<ApiTokenPermissions>().0;

	let input_resource_type = create_rw_signal("".to_string());
	let input_apply_to = create_rw_signal("all".to_string());
	let input_resources = create_rw_signal::<Vec<String>>(vec![]);
	let input_permissions = create_rw_signal::<Vec<String>>(vec![]);

	let resource_type_values = ResourceType::VARIANTS
		.iter()
		.map(|v| InputDropdownOption {
			id: v.to_string(),
			label: v.to_case(Case::Title).to_string(),
			disabled: false,
		})
		.collect::<Vec<InputDropdownOption>>();

	let show_resource_type = create_memo(move |_| {
		match ResourceType::from_str(input_resource_type.get().to_case(Case::Camel).as_str()) {
			Ok(ResourceType::DnsRecord) => false,
			Ok(ResourceType::Workspace) => false,
			_ => true,
		}
	});

	let show_resources = move || {
		show_resource_type.get() &&
			ApplyToOptions::from_str(input_apply_to.get().as_str())
				.is_ok_and(|option| !matches!(option, ApplyToOptions::AllResource))
	};

	let on_select_permission = move |ev: MouseEvent| {
		ev.prevent_default();
		let mut resource_ids = BTreeSet::<Uuid>::new();
		let mut resource_permissions_new = BTreeMap::<Uuid, ResourcePermissionType>::new();

		input_resources.with(|resources| {
			resources.iter().for_each(|resource| {
				if let Ok(parsed_resource_id) = Uuid::parse_str(resource) {
					resource_ids.insert(parsed_resource_id);
				}
			});
		});

		let permissions = input_permissions
			.get()
			.iter()
			.filter_map(|x| match Uuid::parse_str(x) {
				Ok(y) => Some(y),
				Err(_) => None,
			})
			.collect::<Vec<_>>();

		let permission_types = match ApplyToOptions::from_str(input_apply_to.get().as_str()) {
			Ok(ApplyToOptions::Specific) => ResourcePermissionType::Include(resource_ids.clone()),
			Ok(ApplyToOptions::Except) => ResourcePermissionType::Exclude(resource_ids.clone()),
			Ok(ApplyToOptions::AllResource) => {
				ResourcePermissionType::Exclude(BTreeSet::<Uuid>::new())
			}
			Err(_) => ResourcePermissionType::Include(BTreeSet::<Uuid>::new()),
		};
		permissions.iter().for_each(|r| {
			resource_permissions_new.insert(r.to_owned(), permission_types.clone());
		});

		api_token_permissions.update(|permissions| {
			permissions.as_mut().map(|permissions| {
				permissions.insert(
					workspace_id.get(),
					WorkspacePermission::Member {
						permissions: resource_permissions_new.clone(),
					},
				)
			});
		});
	};

	view! {
		<div class="w-full flex items-start justify-start">
			<div class={div_class}>
				<div class="flex flex-col items-start justify-start">
					<InputDropdown
						placeholder={"Select Resource Type".to_string()}
						options={resource_type_values}
						value={input_resource_type}
					/>
				</div>

				<Show when={move || show_resource_type.get()}>
					<div class="flex flex-col items-start justify-start">
						<InputDropdown
							placeholder={format!(
								"All/Specific {}",
								input_resource_type
									.with(|resource| {
										if resource.is_empty() {
											"Resource".to_string()
										} else {
											resource.to_owned()
										}
									}),
							)}
							value={input_apply_to}
							options={vec![
								InputDropdownOption {
									id: "All".to_string(),
									label: format!("All {}s", input_resource_type.get()),
									disabled: false,
								},
								InputDropdownOption {
									id: "Specific".to_string(),
									label: format!("Specific {}", input_resource_type.get()),
									disabled: false,
								},
								InputDropdownOption {
									id: "Except".to_string(),
									label: format!("All {}s Except", input_resource_type.get())
										.to_string(),
									disabled: false,
								},
							]}
						/>
					</div>
				</Show>

				<Show when={show_resources}>
					<ResourceDropdownOptions
						input_resource_type={input_resource_type}
						input_resources={input_resources}
					/>
				</Show>

				<div>
					<PermissionsDropdown
						workspace_id={workspace_id}
						input_permissions={input_permissions}
						input_resource_type={input_resource_type}
					/>
				</div>
			</div>

			<div class="flex items-center justify-center pl-md">
				<button
					on:click={on_select_permission}
					class="flex items-center justify-center br-sm p-xs btn btn-primary"
				>
					<Icon icon={IconType::Plus} color={Color::Secondary} />
				</button>
			</div>
		</div>
	}
}
