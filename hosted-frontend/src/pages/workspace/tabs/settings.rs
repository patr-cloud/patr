use leptos_use::{use_cookie, utils::FromToStringCodec};
use models::api::workspace::Workspace;

use crate::prelude::*;

#[component]
fn ShowWorkspaceInfo(
	/// The workspace data to show
	#[prop(into)]
	workspace: MaybeSignal<WithId<Workspace>>,
) -> impl IntoView {
	view! {
		<div class="fc-fs-fs full-width">
			<div class="flex my-xs full-width">
				<div class="flex-col-2 fr-fs-fs mt-sm">
					<label html_for="workspaceId" class="txt-white txt-sm fr-fs-ct">
						"Workspace ID"
					</label>
				</div>
				<div class="flex-col-10 txt-grey bg-secondary-light br-sm py-xxs px-xl fr-sb-ct">
					<div class="px-sm">{workspace.get().id.to_string()}</div>
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
						value={workspace.get().data.name}
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
	}
}

#[component]
pub fn ManageWorkspaceSettingsTab() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>("access_token");
	let access_token_signal = move || access_token.get();
	let workspace_list = create_resource(access_token_signal, move |value| async move {
		list_user_workspace(value).await
	});

	logging::log!("{:#?}", workspace_list.get());
	view! {
		<div class="fc-fs-fs full-width full-height fit-wide-screen mx-auto px-md my-xl">
			<Transition>
				{
					move || match workspace_list.get() {
						Some(workspace_list) => {
							match workspace_list {
								Ok(data) => {
									view! {
										<ShowWorkspaceInfo workspace={data.clone().workspaces[0].clone()}/>
									}.into_view()
								},
								Err(_) => view! {}.into_view()
							}
						},
						None => view! {}.into_view()
					}
				}
			</Transition>
		</div>
	}
}
