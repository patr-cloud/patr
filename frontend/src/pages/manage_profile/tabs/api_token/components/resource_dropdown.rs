use std::str::FromStr;

use convert_case::{Case, Casing};
use ev::MouseEvent;
use leptos_query::QueryResult;
use models::rbac::ResourceType;
use strum::VariantNames;

use crate::{
	prelude::*,
	queries::{get_deployment_query, list_deployments_query, AllDeploymentsTag},
};

#[component]
pub fn ResourceDropdownOptions(
	/// The Resource Type
	input_resource_type: RwSignal<String>,
	/// The Selected Resources
	input_resources: RwSignal<Vec<String>>,
) -> impl IntoView {
	let QueryResult {
		data: deployments_list,
		..
	} = list_deployments_query().use_query(move || AllDeploymentsTag(0));

	let resource_dropdown_options = create_memo(move |_| {
		let resource_type =
			{ ResourceType::from_str(input_resource_type.get().to_case(Case::Camel).as_str()) };

		match resource_type {
			Ok(ResourceType::Deployment) => deployments_list.get().map(|x| {
				x.map(|x| {
					x.1.deployments
						.iter()
						.map(|x| InputDropdownOption {
							id: x.id.to_string().clone(),
							disabled: false,
							label: x.name.clone(),
						})
						.collect::<Vec<InputDropdownOption>>()
				})
			}),
			Ok(_) => {
				logging::log!("here");
				None
			}
			Err(_) => {
				logging::log!("here");
				None
			}
		}
	});

	view! {
		<Transition>
			<CheckboxDropdown
				placeholder={
					format!(
						"Select {}",
						input_resource_type
							.with(|resource| {
								if resource.is_empty() {
									"Resources".to_string()
								} else {
									resource.to_owned()
								}
							}),
						)
				}
				value={Signal::derive(move || input_resources.get())}
				options={Signal::derive(move || match resource_dropdown_options.get() {
					Some(Ok(options)) => options,
					_ => vec![],
				})}
				on_select={move |(_, id): (MouseEvent, String)| {
					if input_resources.get().iter().any(|e| e.to_owned() == id)
					{
						input_resources
							.update(|options| options.retain(|e| e.to_owned() != id));
					} else {
						input_resources.update(|options| options.push(id.clone()));
					}
				}}
			/>
		</Transition>
	}
}
