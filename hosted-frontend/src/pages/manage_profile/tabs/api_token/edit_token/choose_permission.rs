use std::{
	collections::{BTreeMap, BTreeSet},
	str::FromStr,
	string::ParseError,
};

use convert_case::{Case, Casing};
use ev::MouseEvent;
use leptos_use::utils::FromToStringCodec;
use models::{
	api::workspace::rbac::ListAllPermissionsResponse,
	rbac::{ResourcePermissionType, ResourceType, WorkspacePermission},
};
use server_fn::ServerFn;
use strum::{EnumString, VariantNames};

use crate::{pages::ApiTokenInfo, prelude::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, VariantNames)]
#[strum(serialize_all = "camelCase")]
pub enum ApplyToOptions {
	AllResource,
	Specific,
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
	/// The workspace id of the workspace
	#[prop(into)]
	workspace_id: MaybeSignal<Uuid>,
) -> impl IntoView {
	let div_class = class.with(|cname| {
		format!(
			"gap-sm grid grid-col-{} full-width {}",
			grid_columns.get(),
			cname
		)
	});

	let api_token = expect_context::<ApiTokenInfo>().0;

	let resource_permissions = create_rw_signal(BTreeMap::<Uuid, ResourcePermissionType>::new());

	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

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
		move || (access_token.get(), Some(workspace_id.get().to_string())),
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
					let resource_type = split_name.get(0).map(|x| x.to_owned()).unwrap_or_default();

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
		let mut resources_new = BTreeSet::<Uuid>::new();
		let mut resource_permissions_new = BTreeMap::<Uuid, ResourcePermissionType>::new();

		input_resources.with(|resources| {
			resources.iter().for_each(|resource| {
				if let Ok(parsed_resource_id) = Uuid::parse_str(resource) {
					resources_new.insert(parsed_resource_id);
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
			Ok(ApplyToOptions::Specific) => ResourcePermissionType::Include(resources_new.clone()),
			Ok(ApplyToOptions::Except) => ResourcePermissionType::Exclude(resources_new.clone()),
			Ok(ApplyToOptions::AllResource) => {
				ResourcePermissionType::Exclude(BTreeSet::<Uuid>::new())
			}
			Err(_) => ResourcePermissionType::Include(BTreeSet::<Uuid>::new()),
		};
		permissions.iter().for_each(|r| {
			resource_permissions_new.insert(r.to_owned(), permission_types.clone());
		});
		resource_permissions.set(resource_permissions_new);

		api_token.update(|token| {
			token.as_mut().and_then(|token| {
				token
					.data
					.permissions
					.insert(workspace_id.get(), WorkspacePermission::SuperAdmin);

				Some(())
			});
		})
	};

	create_effect(move |_| {
		logging::log!("resource_type: {}", input_resource_type.get());
		logging::log!("show_resource_type: {}", show_resource_type.get());
		logging::log!(
			"input_apply_to: {:?}",
			ApplyToOptions::from_str(input_apply_to.get().as_str())
		);
		logging::log!("input_resource: {:?}", input_resources.get());
		logging::log!("input_permissions: {:?}", input_permissions.get());
		logging::log!("resources {:?}", resource_permissions.get());
		logging::log!("\n");
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
									value={input_resources}
									options={match resource_dropdown_options.get() {
										Some(Ok(options)) => options,
										_ => vec![]
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
							value={input_permissions}
						/>
					</Transition>
				</div>
			</div>

			<div class="fr-ct-ct pl-md">
				<button
					on:click={on_select_permission}
					class="fr-ct-ct br-sm p-xs btn btn-primary"
				>
					<Icon icon={IconType::Plus} color=Color::Secondary />
				</button>
			</div>
		</div>
	}
}
