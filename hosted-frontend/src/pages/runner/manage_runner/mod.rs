mod head;

pub use self::head::*;
use crate::prelude::*;

#[component]
pub fn ManageRunner() -> impl IntoView {
	view! {
		<RunnerManageHead />
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
							value={"{runner_name}".to_string()}
						/>
					</div>
				</div>
			</form>
		</ContainerBody>
	}
}
