use crate::imports::*;

#[component]
pub fn SkeletonDeploymentCard(
	/// Additional class names to apply to the outer div, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || {
		format!(
			"bg-secondary-light br-sm p-lg fc-fs-fs deployment-card {}",
			class.get(),
		)
	};

	view! {
		<div
			class=class
		>
			<div class="fr-fs-ct full-width px-xxs">
				<h4 class="half-width">
					<Skeleton class="full-width skeleton-div-sm" />
				</h4>
			</div>

			<div class="fr-fs-fs full-width my-auto py-xxs f-wrap">
				{
					vec![1, 2, 3, 4, 5, 6].into_iter().map(|i| view! {
						<div class="half-width p-xxs">
							<Skeleton class="full-width skeleton-div-sm" />
						</div>
					}).collect_view()
				}
			</div>

			<div class="fr-sb-ct full-width px-xxs">
				<div class="half-width">
					<Skeleton class="skeleton-button" />
				</div>

				<div class="half-width">
					<Skeleton class="skeleton-text-half-width" />
				</div>
			</div>
		</div>
	}
}
