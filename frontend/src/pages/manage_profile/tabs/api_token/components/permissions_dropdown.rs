use std::str::FromStr;

use convert_case::{Case, Casing};
use ev::MouseEvent;
use leptos_query::QueryResult;
use models::rbac::ResourceType;
use strum::VariantNames;

use super::ParsedPermission;
use crate::{prelude::*, queries::get_all_permissions_query};

/// Dropdown that displays all the permissions available for the resource type
#[component]
pub fn PermissionsDropdown(
	/// The workspace id of the workspace
	#[prop(into)]
	workspace_id: MaybeSignal<Uuid>,
	/// The Input Permissions
	input_permissions: RwSignal<Vec<String>>,
	/// Input Resource Type
	input_resource_type: RwSignal<String>,
) -> impl IntoView {
	let QueryResult {
		data: all_permissions,
		..
	} = get_all_permissions_query().use_query(move || workspace_id.get());

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

	let permission_options = Signal::derive(move || {
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

	view! {
		<Transition>
			<CheckboxDropdown
				placeholder={"Select Permissions".to_string()}
				options={permission_options}
				value={Signal::derive(move || input_permissions.get())}
				on_select={move |(_, id): (MouseEvent, String)| {
					if input_permissions.get().iter().any(|e| e.to_owned() == id) {
						input_permissions
							.update(|options| options.retain(|e| e.to_owned() != id));
					} else {
						input_permissions.update(|options| options.push(id.clone()));
					}
				}}
			/>
		</Transition>
	}
}
