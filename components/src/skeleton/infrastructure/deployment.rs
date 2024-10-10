use crate::imports::*;

#[component]
pub fn DeploymentSkeletonCard(
	/// Additional class names to apply to the outer div, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || {
		format!(
			"bg-secondary-light rounded-sm p-lg flex-col items-start justify-start deployment-card {}",
			class.get(),
		)
	};

	view! {
		<div class={class}>
			<div class="flex justify-start items-center w-full px-xxs">
				<h4 class="w-1/2">
					<Skeleton class="full-width skeleton-div-sm"/>
				</h4>
			</div>

			<div class="flex items-start justify-start w-full my-auto py-xxs f-wrap">

				{(0..5).collect::<Vec<_>>()
					.into_iter()
					.map(|_| {
						view! {
							<div class="w-1/2 p-xxs">
								<Skeleton class="full-width skeleton-div-sm"/>
							</div>
						}
					})
					.collect_view()}

			</div>

			<div class="flex justify-between items-center w-full px-xxs">
				<div class="w-1/2">
					<Skeleton class="skeleton-button"/>
				</div>

				<div class="w-1/2">
					<Skeleton class="skeleton-text-half-width"/>
				</div>
			</div>
		</div>
	}
}
