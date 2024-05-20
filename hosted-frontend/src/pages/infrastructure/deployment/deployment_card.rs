use super::DeploymentType;
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
	deployment: MaybeSignal<DeploymentType>,
	/// Additional Classes to add to the outer div, if any.:w
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || {
		format!(
			"bg-secondary-light br-sm p-lg fc-fs-fs deployment-card {}",
			class.get()
		)
	};

	let items = vec![
		DeploymentCardItem {
			label: "REGISTRY",
			value: deployment.get().id,
		},
		DeploymentCardItem {
			label: "REGION",
			value: deployment.get().region,
		},
		DeploymentCardItem {
			label: "REPOSITORY",
			value: "registry.patr.cloud".to_owned(),
		},
		DeploymentCardItem {
			label: "IMAGE TAG",
			value: deployment.get().image_tag,
		},
		DeploymentCardItem {
			label: "MACHINE TYPE",
			value: deployment.get().machine_type,
		},
	];

	view! {
		<div class=class>
			<div class="fr-fs-ct gap-md full-width px-xxs">
				<h4 class="txt-md txt-primary w-25 txt-of-ellipsis of-hidden">
					{deployment.get().name}
				</h4>

				<StatusBadge status=deployment.get().status/>
			</div>

			<div class="fr-fs-fs txt-white full-width f-wrap my-auto">

				{items
					.into_iter()
					.map(|item| {
						view! {
							<div class="half-width p-xxs">
								<div class="bg-secondary-medium br-sm px-lg py-sm fc-ct-fs">
									<span class="letter-sp-md txt-xxs txt-grey">{item.label}</span>
									<span class="txt-primary w-15 txt-of-ellipsis of-hidden">
										{item.value}
									</span>
								</div>
							</div>
						}
					})
					.collect::<Vec<_>>()} <div class="half-width p-xxs">
					<Link class="bg-secondary-medium br-sm px-lg py-sm fc-ct-fs full-width">
						<span class="letter-sp-md txt-xxs txt-grey">"LIVE LINKS"</span>
						<span class="txt-primary w-15 txt-of-ellipsis of-hidden fr-fs-ct">
							"PUBLIC URL"
							<Icon
								icon=IconType::ArrowUpRight
								color=Color::Primary
								size=Size::ExtraSmall
							/>
						</span>
					</Link>
				</div>
			</div>

			<div class="fr-sb-ct mt-xs full-width px-xxs">
				<Link style_variant=LinkStyleVariant::Contained>
					<Icon
						icon=IconType::PlayCircle
						size=Size::ExtraSmall
						color=Color::Secondary
						class="mr-xs"
					/>
					"START"
				</Link>

				<Link
					r#type=Variant::Link
					to=format!("{}", deployment.get().id)
					class="letter-sp-md txt-sm fr-fs-ct"
				>
					"Manage Deployment"
					<Icon icon=IconType::ChevronRight size=Size::ExtraSmall color=Color::Primary/>
				</Link>
			</div>
		</div>
	}
}
