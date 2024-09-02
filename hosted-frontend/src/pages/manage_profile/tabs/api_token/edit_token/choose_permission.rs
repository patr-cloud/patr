use std::{
	collections::{BTreeMap, BTreeSet},
	str::FromStr,
};

use convert_case::{Case, Casing};
use ev::MouseEvent;
use models::rbac::{ResourcePermissionType, ResourceType, WorkspacePermission};
use strum::VariantNames;

use crate::{pages::ApiTokenInfo, prelude::*};

/// Enum that specifies whether to apply the permission to all resources, a
/// specific set of resources, or all resources except a specific set of
/// resources.
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
struct ParsedPermission {
	/// The name of the permission
	name: String,
	/// The Resource Type
	resource_type: ResourceType,
	/// Details of the permission
	details: String,
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
	let div_class = class.with(|cname| {
		format!(
			"gap-sm grid grid-col-{} w-full {}",
			grid_columns.get(),
			cname
		)
	});

	let api_token = expect_context::<ApiTokenInfo>().0;

	let access_token = move || AuthState::load().0.get().get_access_token();

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

	let all_permissions = create_resource(
		move || (access_token(), workspace_id.get()),
		move |(access_token, workspace_id)| async move {
			list_all_permissions(access_token, workspace_id).await
		},
	);

	let filtered_permissions = create_memo(move |_| {
		let permissions = all_permissions.get();
		match permissions {
			Some(Ok(permissions)) => permissions
				.permissions
				.iter()
				.filter_map(|permission| {
					let split_name = permission.name.split("::").collect::<Vec<_>>();
					let resource_type =
						split_name.first().map(|x| x.to_owned()).unwrap_or_default();

					let permission_name =
						split_name.get(1).map(|x| x.to_owned()).unwrap_or_default();
					let resource_type =
						ResourceType::from_str(resource_type.to_case(Case::Camel).as_str());
					match resource_type {
						Ok(resource_type) => Some(WithId {
							id: permission.id,
							data: ParsedPermission {
								name: permission_name.to_string(),
								resource_type,
								details: permission.description.clone(),
							},
						}),
						Err(_) => None,
					}
				})
				.filter(|x| {
					match ResourceType::from_str(
						input_resource_type.get().to_case(Case::Camel).as_str(),
					) {
						Ok(r_type) => r_type == x.resource_type,
						_ => false,
					}
				})
				.collect::<Vec<WithId<ParsedPermission>>>(),
			_ => {
				logging::log!("error fetching permissions");
				vec![]
			}
		}
	});

	let permissions_options = Signal::derive(move || {
		filtered_permissions
			.get()
			.iter()
			.map(|x| InputDropdownOption {
				id: x.id.to_string(),
				label: x.name.clone(),
				disabled: false,
			})
			.collect::<Vec<InputDropdownOption>>()
	});

	let deployments_list = get_deployments();
	let resource_dropdown_options = create_memo(move |_| {
		let resource_type =
			{ ResourceType::from_str(input_resource_type.get().to_case(Case::Camel).as_str()) };

		match resource_type {
			Ok(ResourceType::Deployment) => deployments_list.get().map(|x| {
				x.map(|x| {
					x.deployments
						.iter()
						.map(|x| InputDropdownOption {
							id: x.id.to_string().clone(),
							disabled: false,
							label: x.name.clone(),
						})
						.collect::<Vec<InputDropdownOption>>()
				})
			}),
			Ok(_) => None,
			Err(_) => None,
		}
	});

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

		logging::log!("filtered_permissions {:?}", filtered_permissions.get());
		api_token.update(|token| {
			if let Some(token) = token.as_mut() {
				token.data.permissions.insert(
					workspace_id.get(),
					WorkspacePermission::Member {
						permissions: resource_permissions_new.clone(),
					},
				);
			}
		})
	};

	view! {
		<div class="w-full flex items-start justify-start">
			<div class={div_class}>
				<div class="flex flex-col items-start justify-start">
					<InputDropdown
						placeholder="Select Resource Type".to_string()
						options={resource_type_values}
						value={input_resource_type}
					/>
				</div>

				<Show when={move || show_resource_type.get()}>
					<div class="flex flex-col items-start justify-start">
						<InputDropdown
							placeholder={format!("All/Specific {}", input_resource_type.with(|resource|
								if resource.is_empty() {"Resource".to_string()} else {resource.to_owned()}
							))}
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
									label: format!("All {}s Except", input_resource_type.get() ).to_string(),
									disabled: false,
								},
							]}
						/>
					</div>
				</Show>

				<Transition>
					<Show when={show_resources}>
						{
							move || view! {
								<CheckboxDropdown
									placeholder={format!("Select {}", input_resource_type.with(|resource|
										if resource.is_empty() {"Resources".to_string()} else {resource.to_owned()}
									))}
									value={Signal::derive(move || input_resources.get())}
									options={match resource_dropdown_options.get() {
										Some(Ok(options)) => options,
										_ => vec![]
									}}
									on_select={move |(_, id): (MouseEvent, String)| {
										if input_resources.get().iter().any(|e| e.to_owned() == id) {
											input_resources.update(|options| options.retain(|e| e.to_owned() != id));
										} else {
											input_resources.update(|options| options.push(id.clone()));
										}
									}}
								/>
							}
						}
					</Show>
				</Transition>

				<div>
					<Transition>
						<CheckboxDropdown
							placeholder="Select Permissions".to_string()
							options={permissions_options}
							value={Signal::derive(move || input_permissions.get())}
							on_select={move |(_, id): (MouseEvent, String)| {
								if input_permissions.get().iter().any(|e| e.to_owned() == id) {
									input_permissions.update(|options| options.retain(|e| e.to_owned() != id));
								} else {
									input_permissions.update(|options| options.push(id.clone()));
								}
							}}
						/>
					</Transition>
				</div>
			</div>

			<div class="flex items-center justify-center pl-md">
				<button
					on:click={on_select_permission}
					class="flex items-center justify-center br-sm p-xs btn btn-primary"
				>
					<Icon icon={IconType::Plus} color=Color::Secondary />
				</button>
			</div>
		</div>
	}
}
