use crate::{pages::*, prelude::*};

#[component]
pub fn ManageDeploymentScaling() -> impl IntoView {
	view! {
		<div class="fc-fs-fs full-width px-xl mt-xl txt-white txt-sm fit-wide-screen mx-auto gap-md">
			<div class="flex full-width">
				<div class="flex-col-2 my-auto pr-md">
					<span class="txt-sm">"Choose Horizontal Scale"</span>
				</div>

				<div class="flex-col-10 fc-fs-ct bg-secondary-light p-xl br-sm">
					<p class="full-width letter-sp-md mb-lg txt-xxs">"Choose the minimum and maximum number of instances for your deployment "</p>

					<div class="full-width fr-ct-ct">
						<div class="flex-col-2 fc-ct-ct">
							<label html_for="minHorizontalScale">"Minimum Scale"</label>

							<NumberPicker value=5 style_variant=SecondaryColorVariant::Medium />
						</div>

						<div class="flex-col-8 mt-xl px-xl fc-fs-ct">
							<DoubleInputSlider
								class="full-width"
							/>

							<p class="txt-warning txt-xxs">
								"Any excess volumes will be removed if the number of instances is reduced."
							</p>
						</div>

						<div class="flex-col-2 fc-ct-ct">
							<label html_for="maxHorizontalScale">"Maximum Scale"</label>

							<NumberPicker value=4 style_variant=SecondaryColorVariant::Medium />
						</div>
					</div>
				</div>
			</div>

			<div class="flex full-width">
				<div class="flex-col-2 my-auto pr-md">
					<span class="txt-sm">"Manage Resource Allocation"</span>
				</div>

				<div class="flex-col-10 fr-fs-ct of-auto">
					<div class="full-width p-xl br-sm bg-secondary-light fc-fs-fs of-auto">
						<p class="letter-sp-md mb-lg txt-xxs">
							"Specify the resources to be allocated to your container"
						</p>

						<div class="fr-fs-ct ofx-auto py-xxs gap-md">
							<MachineTypeCard />
							<MachineTypeCard />
							<MachineTypeCard />
						</div>
					</div>
				</div>
			</div>

			<div class="flex full-width">
				<div class="flex-col-2 my-auto pr-md">
					<span class="txt-sm">"Estimated Cost"</span>
				</div>

				<div class="flex-col-10 fc-fs-fs of-auto">
					<div class="fr-fs-ct">
							<span class="txt-xl txt-success txt-thin">
								"$5"
								<small class="txt-grey txt-lg">"/month"</small>
							</span>
					</div>

					<p class="txt-grey">
						"This deployment is eligible for "
						<strong class="txt-medium txt-sm">"Free"</strong> plan
						"since it's your first deployment and"
						<br />
						"you have selected the base machine type with only one instance."
					</p>
				</div>
			</div>
		</div>
	}
}
