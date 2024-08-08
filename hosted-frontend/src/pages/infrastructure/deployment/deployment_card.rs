use models::api::workspace::deployment::Deployment;

use crate::prelude::*;

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

				<StatusBadge status={
					let deployment = deployment.clone();
					Signal::derive(move || Some(
						Status::from_deployment_status(deployment.get().status.clone()),
					))
				} />
			</div>

			<div class="flex items-start justify-start text-white w-full flex-wrap">
				{
					items.into_iter()
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
				<Link style_variant={LinkStyleVariant::Contained}>
					<Icon
						icon={IconType::PlayCircle}
						size={Size::ExtraSmall}
						color={Color::Secondary}
						class="mr-xs"
					/>
					"START"
				</Link>

				<Link
					r#type={Variant::Link}
					to={deployment.get().id.to_string()}
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
