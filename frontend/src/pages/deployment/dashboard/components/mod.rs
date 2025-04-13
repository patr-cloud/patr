use crate::prelude::*;

/// A Deployment Card Item Type for the list of options,
#[derive(Clone)]
pub struct DeploymentCardItem {
	/// The Label of the deployment
	label: &'static str,
	/// The Value of the deployment
	value: &'static str,
}

#[component]
pub fn DeploymentCard(
	/// Additional Classes to add to the outer div, if any.:w
	#[prop(into, optional)]
	class: Signal<String>,
) -> impl IntoView {
	let class = move || {
		format!(
			"bg-secondary-light rounded-sm p-lg flex flex-col items-start justify-between gap-md deployment-card {}",
			class.get()
		)
	};

	let items = Signal::derive(move || {
		vec![
			DeploymentCardItem {
				label: "REGISTRY",
				value: "registry url",
			},
			DeploymentCardItem {
				label: "REPOSITORY",
				value: "Repo Id",
			},
			DeploymentCardItem {
				label: "IMAGE TAG",
				value: "Image Tag",
			},
			DeploymentCardItem {
				label: "MACHINE TYPE",
				value: "Machine Type",
			},
		]
	});

	view! {
		<div class={class}>
			<div class="flex items-start justify-start gap-md w-full px-xxs">
				<h4 class="text-md text-primary text-ellipsis overflow-hidden">
					"Deployment Name"
				</h4>

				<StatusBadge status={Status::Created} />
			</div>

			<div class="deployment-card-items grid-cols-[1fr_1fr] text-white w-full">
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
			</div>
		</div>
	}
}
