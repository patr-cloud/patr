mod head;

pub use self::head::*;
use crate::prelude::*;

#[component]
pub fn CreateRunner() -> impl IntoView {
	view! {
		<RunnerCreateHead />
		<ContainerBody class="p-xs px-md gap-md ofy-auto txt-white">
			<form class="full-width full-height px-md py-xl fc-fs-fs fit-wide-screen mx-auto gap-md">
				<div class="flex full-width">
					<div class="flex-col-2 fr-fs-fs pt-sm">
						<label html_for="name" class="txt-white txt-sm">
							"Runner Name"
						</label>
					</div>

					<div class="flex-col-10 fc-fs-fs">
						<Input
							id="name"
							name="name"
							r#type={InputType::Text}
							placeholder="Enter runner name"
							class="full-width"
						/>
					</div>
				</div>

				<div class="flex full-width">
					<div class="flex-col-2 fr-fs-fs pt-sm">
						<label html_for="name" class="txt-white txt-sm">
							"Machine Type"
						</label>
					</div>

					<div class="flex-col-10 fc-fs-fs of-auto">
						<div class="full-width p-xl br-sm bg-secondary-light fc-fs-fs of-auto">
							<span class="letter-sp-md mb-lg txt-xxs">
								"Specify the resources to be allocated to your container"
							</span>

							<div class="fr-fs-ct ofx-auto p-xxs gap-xs">

							</div>
						</div>
					</div>
				</div>

			</form>
		</ContainerBody>
	}
}
