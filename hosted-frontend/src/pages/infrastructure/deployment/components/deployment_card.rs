use std::rc::Rc;

use convert_case::*;
use ev::MouseEvent;
use models::api::workspace::deployment::{Deployment, DeploymentStatus};

use crate::{
	prelude::*,
	queries::{start_deployment_query, stop_deployment_query},
};

/// A Deployment Card Item Type for the list of options,
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
			"bg-secondary-light rounded-sm p-lg flex flex-col items-start justify-between deployment-card {}",
			class.get()
		)
	};

	let items = vec![
		DeploymentCardItem {
			label: "REGISTRY",
			value: deployment.get().id.to_string(),
		},
		DeploymentCardItem {
			label: "RUNNER",
			value: deployment.get().runner.to_string(),
		},
		DeploymentCardItem {
			label: "REPOSITORY",
			value: "registry.patr.cloud".to_owned(),
		},
		DeploymentCardItem {
			label: "IMAGE TAG",
			value: deployment.get().image_tag.clone(),
		},
		DeploymentCardItem {
			label: "MACHINE TYPE",
			value: deployment.get().machine_type.to_string(),
		},
	];

	view! {
		<div class={class}>
			<div class="fr-fs-ct gap-md w-full px-xxs">
				<h4 class="text-md text-primary text-ellipsis overflow-hidden">
					{deployment.get().name.clone()}
				</h4>

				<StatusBadge
					status={
						let deployment = deployment.clone();
						Signal::derive(move || Some(
							Status::from_deployment_status(deployment.get().status.clone()),
						))
					}
				/>
			</div>

			<div class="flex items-start justify-start text-white w-full flex-wrap">
				{
					items
						.into_iter()
						.map(|item| {
							view! {
								<div class="w-1/2 p-xxs">
									<div class="bg-secondary-medium rounded-sm px-lg py-sm flex flex-col items-start justify-center">
										<span class="tracking-[1px] text-xxs text-grey">
											{item.label}
										</span>
										<span class="text-primary w-[15ch] text-ellipsis overflow-hidden">
											{item.value}
										</span>
									</div>
								</div>
							}
						})
						.collect::<Vec<_>>()
				}

				<div class="w-1/2 p-xxs">
					<a
						href=""
						class="bg-secondary-medium rounded-sm px-lg py-sm flex flex-col items-start justify-center w-full"
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
				</div>
			</div>

			<div class="flex justify-between items-center mt-xs w-full px-xxs">
				<Link
					disabled={
						store_deployment.with_value(
							move |deployment|
								deployment.get().status.clone() == DeploymentStatus::Deploying ||
								deployment.get().status.clone() == DeploymentStatus::Errored ||
								deployment.get().status.clone() == DeploymentStatus::Unreachable
						)
					}
					style_variant={LinkStyleVariant::Contained}
				>
					{
						let deployment = store_deployment.with_value(move |deployment| deployment.get());
						match deployment.status.clone() {
							DeploymentStatus::Running => {
								view! {
									<Icon
										icon={IconType::PauseCircle}
										size={Size::ExtraSmall}
										color={Color::Secondary}
										class="mr-xs"
									/>
								}.into_view()
							}
							_ => {
								view! {
									<Icon
										icon={IconType::PlayCircle}
										size={Size::ExtraSmall}
										color={Color::Secondary}
										class="mr-xs"
									/>
								}.into_view()
							}
						}
					}
					{
						format!(
							"{}",
							store_deployment
								.with_value(
									move |deployment| deployment
										.get()
										.status
										.to_string()
										.to_case(Case::Title)
								)
						).into_view()
					}
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
