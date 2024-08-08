use crate::prelude::*;

#[component]
pub fn DatabasePlanCard() -> impl IntoView {
	view! {
		<div class="px-xl py-lg bg-secondary-light br-sm fc-fs-fs database-plan-card cursor-pointer txt-white ">
			<span class="txt-xxs">
				<strong class="txt-bold txt-primary">"Free"</strong>
				" on BYOC"
			</span>

			<div class="fr-fs-bl">
				<span class="txt-lg">"1 GB"</span>
				<span class="txt-disabled ml-xxs txt-xxs">"RAM"</span>
			</div>
			<div class="fr-fs-bl">
				<span class="txt-lg">"4"</span>
				<span class="txt-disabled ml-xxs txt-xxs">"vCPU"</span>
			</div>
			<div class="fr-fs-bl">
				<span class="txt-lg">"30 GB"</span>
				<span class="txt-disabled ml-xxs txt-xxs">"Volume"</span>
			</div>
		</div>
	}
}
