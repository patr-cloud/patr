use crate::prelude::*;

#[component]
pub fn ManageWorkspaceSettingsTab() -> impl IntoView {
	view! {
		<div class="fc-fs-fs full-width full-height fit-wide-screen mx-auto px-md my-xl">
			<div class="fc-fs-fs full-width">
				<div class="flex my-xs full-width">
					<div class="flex-col-2 fr-fs-fs mt-sm">
						<label html_for="workspaceId" class="txt-white txt-sm fr-fs-ct">
							"Workspace ID"
						</label>
					</div>
					<div class="flex-col-10 txt-grey bg-secondary-light br-sm py-xxs px-xl fr-sb-ct">
						<div class="px-sm">"currentWorkspace"</div>
						<button
							class="btn-icon ml-auto p-xxs"
							aria_label="Copy workspace id"
						>
							<Icon icon=IconType::Copy size=Size::ExtraSmall />
						</button>
						// {copy ? (
						// 	<Icon icon="check" class="ml-auto m-xxs" size="xs" />
						// ) : (
						// )}
					</div>
				</div>

				<div class="flex my-xs full-width">
					<div class="flex-col-2 fr-fs-fs mt-sm">
						<label html_for="name" class="txt-white txt-sm fr-fs-ct">
							"Name"
						</label>
					</div>
					<div class="flex-col-10 fr-fs-fs">
						<Input
							placeholder="Workspace Name"
							class="full-width"
							r#type=InputType::Text
							id="name"
							name="name"
						/>
						// {copy ? (
						// 	<Icon icon="check" class="ml-auto m-xxs" size="xs" />
						// ) : (
						// )}
					</div>
				</div>

				<div class="flex my-xs full-width">
					<div class="flex-col-2 fc-fs-fs mt-md">
						<label html_for="alertEmail" class="txt-white txt-sm fr-fs-ct">
							"Alert Email(s)"
						</label>
						<span class="txt-grey">
							"These are a list of emails that will recieve a notification whenever a deployment crashes"
						</span>
					</div>

					<div class="flex-col-10 fc-fs-fs">
						<div class="full-width flex fr-fs-ct mb-xs">
							<div class="flex-col-11">
								<Textbox value="ac380012@gmail.com".into_view() />
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	}
}
