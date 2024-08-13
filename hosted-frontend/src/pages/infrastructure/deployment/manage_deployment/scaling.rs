use crate::prelude::*;

#[component]
pub fn ManageDeploymentScaling() -> impl IntoView {
	view! {
		<div
			class="flex flex-col items-start justify-start w-full px-xl mt-xl
				text-white text-sm fit-wide-screen mx-auto gap-md"
			>
			<div class="flex w-full">
				<div class="flex-2 my-auto pr-md">
					<span class="text-sm">"Choose Horizontal Scale"</span>
				</div>

				<div class="flex-10 fc-fs-ct flex flex-col items-center justify-start bg-secondary-light p-xl br-sm">
					<p class="w-full tracking-[1px] mb-lg text-xxs">
						"Choose the minimum and maximum number of instances for your deployment "
					</p>

					<div class="w-full flex items-center justify-center">
						<div class="flex-2 flex flex-col items-center justify-center">
							<label html_for="minHorizontalScale">"Minimum Scale"</label>

							<NumberPicker value=5 style_variant={SecondaryColorVariant::Medium}/>
						</div>

						<div class="flex-8 mt-xl px-xl flex flex-col items-center justify-start">
							// <DoubleInputSlider class="w-full"/>

							<p class="text-warning text-xxs">
								"Any excess volumes will be removed if the number of instances is reduced."
							</p>
						</div>

						<div class="flex-2 flex flex-col items-center justify-center">
							<label html_for="maxHorizontalScale">"Maximum Scale"</label>

							<NumberPicker value=4 style_variant={SecondaryColorVariant::Medium}/>
						</div>
					</div>
				</div>
			</div>

			<div class="flex w-full">
				<div class="flex-2 my-auto pr-md">
					<span class="text-sm">"Manage Resource Allocation"</span>
				</div>

				<div class="flex-10 flex items-center justify-start overflow-auto">
					<div
						class="w-full p-xl br-sm bg-secondary-light
						flex flex-col items-start justify-start overflow-auto"
					>
						<p class="tracking-[1px] mb-lg text-xxs">
							"Specify the resources to be allocated to your container"
						</p>

						<div class="flex items-center justify-start overflow-x-auto py-xxs gap-md">
							// <MachineTypeCard/>
							// <MachineTypeCard/>
							// <MachineTypeCard/>
						</div>
					</div>
				</div>
			</div>

			<div class="flex w-full">
				<div class="flex-2 my-auto pr-md">
					<span class="text-sm">"Estimated Cost"</span>
				</div>

				<div class="flex-10 flex flex-col items-start justify-start overflow-auto">
					<div class="flex items-center justify-start">
						<span class="text-xl text-success text-thin">
							"$5" <small class="text-grey text-lg">"/month"</small>
						</span>
					</div>

					<p class="text-grey">
						"This deployment is eligible for "
						<strong class="text-medium text-sm">"Free"</strong> "plan"
						"since it's your first deployment and" <br/>
						"you have selected the base machine type with only one instance."
					</p>
				</div>
			</div>
		</div>
	}
}
