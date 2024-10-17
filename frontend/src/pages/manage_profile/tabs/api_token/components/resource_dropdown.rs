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
	let current_page = create_rw_signal::<usize>(0);
	let QueryResult {
		data: deployments_list,
		..
	} = list_deployments_query().use_query(move || AllDeploymentsTag(current_page.get()));

	let resource_list_options = create_rw_signal::<Vec<InputDropdownOption>>(vec![]);

	create_effect(move |_| {
		let resource_type =
			{ ResourceType::from_str(input_resource_type.get().to_case(Case::Camel).as_str()) };

		match resource_type {
			Ok(ResourceType::Deployment) => match deployments_list.get() {
				Some(Ok((_, deployments_list))) => resource_list_options.update(|resource_list| {
					resource_list.extend(deployments_list.deployments.iter().map(|deployment| {
						InputDropdownOption {
							id: deployment.id.to_string(),
							disabled: false,
							label: deployment.name.clone(),
						}
					}));
				}),
				_ => {}
			},
			Ok(_) => {
				resource_list_options.set(vec![]);
			}
			Err(_) => {
				resource_list_options.set(vec![]);
			}
		}
	});

	// let resource_dropdown_options = create_memo(move |_| {
	// 	let resource_type =
	// 		{ ResourceType::from_str(input_resource_type.get().to_case(Case::Camel).
	// as_str()) };

	// 	match resource_type {
	// 		Ok(ResourceType::Deployment) => deployments_list.get().map(|x| {
	// 			x.map(|x| {
	// 				x.1.deployments
	// 					.iter()
	// 					.map(|x| InputDropdownOption {
	// 						id: x.id.to_string(),
	// 						disabled: false,
	// 						label: x.name.clone(),
	// 					})
	// 					.collect::<Vec<InputDropdownOption>>()
	// 			})
	// 		}),
	// 		Ok(_) => None,
	// 		Err(_) => None,
	// 	}
	// });

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
				options={resource_list_options}
				on_select={move |(_, id): (MouseEvent, String)| {
					if input_resources.get().iter().any(|e| e.to_owned() == id)
					{
						input_resources
							.update(|options| options.retain(|e| e.to_owned() != id));
					} else {
						input_resources.update(|options| options.push(id.clone()));
					}
				}}
				additional_view_class="bg-[#292548]"
				additional_view={view! {
					<button
						class="text-primary w-full justify-center"
						on:click={move |ev| {
							ev.prevent_default();;
							current_page.update(|v| *v += 1)
						}}
					>
						"LOAD MORE"
					</button>
				}.into_view()}
			/>
		</Transition>
	}
}
