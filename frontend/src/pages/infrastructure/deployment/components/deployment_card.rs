use std::rc::Rc;

use convert_case::*;
use ev::MouseEvent;
use models::api::workspace::deployment::{
	Deployment,
	DeploymentRegistry,
	DeploymentStatus,
	ListAllDeploymentMachineTypeResponse,
};

use crate::{
	prelude::*,
	queries::{list_machines_query, start_deployment_query, stop_deployment_query},
};

/// A Deployment Card Item Type for the list of options,
#[derive(Clone)]
pub struct DeploymentCardItem {
	/// The Label of the deployment
	label: &'static str,
	/// The Value of the deployment
	value: String,
}

#[component]
pub fn DeploymentCard(
	/// The Deployment Info
	#[prop(into)]
	deployment: MaybeSignal<WithId<Deployment>>,
	/// Additional Classes to add to the outer div, if any.:w
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let start_deployment_action = start_deployment_query();
	let stop_deployment_action = stop_deployment_query();

	let store_deployment = store_value(deployment.clone());
	let deployment = Signal::derive(move || deployment.get().clone());

	let on_click_start_stop = move |ev: &MouseEvent| {
		ev.prevent_default();
		let status = store_deployment.with_value(move |deployment| deployment.get().status.clone());
		let deployment_id =
			store_deployment.with_value(move |deployment| deployment.get().id.clone());

		match status {
			DeploymentStatus::Running => {
				stop_deployment_action.dispatch(deployment_id);
			}
			DeploymentStatus::Created | DeploymentStatus::Stopped => {
				start_deployment_action.dispatch(deployment_id);
			}
			_ => {}
		}
	};

	let class = move || {
		format!(
			"bg-secondary-light rounded-sm p-lg flex flex-col items-start justify-between gap-md deployment-card {}",
			class.get()
		)
	};

	let machine_list = list_machines_query();

	let machine_type_string = Signal::derive({
		// let deployment = deployment.clone();
		// logging::log!("machine_list: {:?}", deployment.get().machine_type);
		move || match machine_list.get() {
			Some(machine_list) => match machine_list {
				Ok(ListAllDeploymentMachineTypeResponse { machine_types }) => machine_types
					.into_iter()
					.find(|machine| machine.id == deployment.get().machine_type)
					.map(|machine_type| {
						format!(
							"{}vCPU {}MB RAM",
							machine_type.cpu_count, machine_type.memory_count
						)
					})
					.unwrap_or(deployment.get().machine_type.to_string()),
				Err(_) => "Error".to_string(),
			},
			None => "Loading..".to_string(),
		}
	});

	let items = Signal::derive(move || {
		vec![
			DeploymentCardItem {
				label: "REGISTRY",
				value: deployment.get().registry.registry_url().clone(),
			},
			DeploymentCardItem {
				label: "REPOSITORY",
				value: match deployment.get().registry.clone() {
					DeploymentRegistry::PatrRegistry { repository_id, .. } => {
						repository_id.to_string()
					}
					DeploymentRegistry::ExternalRegistry { image_name, .. } => image_name,
				},
			},
			DeploymentCardItem {
				label: "IMAGE TAG",
				value: deployment.get().image_tag.clone(),
			},
			DeploymentCardItem {
				label: "MACHINE TYPE",
				value: machine_type_string.get().clone(),
			},
		]
	});

	view! {
		<div class={class}>
			<div class="fr-fs-ct gap-md w-full px-xxs">
				<h4 class="text-md text-primary text-ellipsis overflow-hidden">
					{move || deployment.get().name.clone()}
				</h4>

				<StatusBadge status={
					let deployment = deployment.clone();
					Signal::derive(move || Some(
						Status::from_deployment_status(deployment.get().status.clone()),
					))
				} />
			</div>

			<div class="deployment-card-items text-white w-full">
				{move || {
					items
						.get()
						.into_iter()
						.map(|item| {
							view! {
								<div class="bg-secondary-medium rounded-sm flex flex-col items-start justify-center">
									<span class="tracking-[1px] text-xxs text-grey">
										{item.label}
									</span>
									<span class="text-primary w-[15ch] h-4 text-ellipsis overflow-hidden">
										{item.value}
									</span>
								</div>
							}
						})
						.collect::<Vec<_>>()
				}}
				<Popover
					trigger_class="bg-secondary-medium rounded-sm flex flex-col items-start justify-center w-full"
					trigger_children={
						view! {
							<a
								href=""
								class="flex flex-col items-start justify-center w-full"
							>
								<span class="tracking-[1px] text-xxs text-grey">"LIVE LINKS"</span>
								<span class="text-primary w-[15ch] text-ellipsis overflow-hidden flex items-center justify-start">
									"PUBLIC URL"
									<Icon
										icon={IconType::ArrowUpRight}
										color={Color::Primary}
										size={Size::ExtraSmall}
									/>
								</span>
							</a>
						}.into_view()
					}
					popover_content={
						view! {
							<div class="tooltip fixed">
								"something in the way"
							</div>
						}.into_view()
					}
				>
				</Popover>
			</div>

			<div class="flex justify-between items-center mt-xs w-full px-xxs">
				<Link
					disabled={store_deployment
						.with_value(move |deployment| {
							deployment.get().status.clone() == DeploymentStatus::Deploying
								|| deployment.get().status.clone() == DeploymentStatus::Errored
								|| deployment.get().status.clone() == DeploymentStatus::Unreachable
						})}
					style_variant={LinkStyleVariant::Contained}
				>
					{
						let deployment = store_deployment
							.with_value(move |deployment| deployment.get());
						match deployment.status.clone() {
							DeploymentStatus::Running => {
								view! {
									<Icon
										icon={IconType::PauseCircle}
										size={Size::ExtraSmall}
										color={Color::Secondary}
										class="mr-xs"
									/>
								}
									.into_view()
							}
							_ => {
								view! {
									<Icon
										icon={IconType::PlayCircle}
										size={Size::ExtraSmall}
										color={Color::Secondary}
										class="mr-xs"
									/>
								}
									.into_view()
							}
						}
					}
					{format!(
						"{}",
						store_deployment
							.with_value(move |deployment| {
								deployment.get().status.to_string().to_case(Case::Title)
							}),
					)
						.into_view()}
				</Link>

				<Link
					r#type={Variant::Link}
					on_click={Rc::new(on_click_start_stop)}
					to={deployment.clone().get().id.to_string()}
					class="leading-[1px] text-sm flex items-start justify-center"
				>
					"Manage Deployment"
					<Icon
						icon={IconType::ChevronRight}
						size={Size::ExtraSmall}
						color={Color::Primary}
					/>
				</Link>
			</div>
		</div>
	}
}
