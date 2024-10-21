use models::api::workspace::Workspace;

use crate::prelude::*;

#[component]
fn ShowWorkspaceInfo(
	/// The workspace data to show
	#[prop(into)]
	workspace: MaybeSignal<WithId<Workspace>>,
) -> impl IntoView {
	view! {
		<div class="flex flex-col items-start justify-start w-full">
			<div class="flex my-xs w-full">
				<div class="flex-2 flex items-start justify-start mt-sm">
					<label
						html_for="workspaceId"
						class="text-white text-sm flex items-center justify-start"
					>
						"Workspace ID"
					</label>
				</div>
				<div class="flex-10 text-grey bg-secondary-light br-sm py-xxs px-xl flex justify-between items-center">
					<div class="px-sm">{workspace.get().id.to_string()}</div>
					<button class="btn-icon ml-auto p-xxs" aria_label="Copy workspace id">
						<Icon icon={IconType::Copy} size={Size::ExtraSmall} />
					</button>
				</div>
			</div>

			<div class="flex my-xs w-full">
				<div class="flex-2 flex items-start justify-start mt-sm">
					<label
						html_for="name"
						class="text-white text-sm flex items-center justify-start"
					>
						"Name"
					</label>
				</div>
				<div class="flex-10 flex items-start justify-start">
					<Input
						placeholder="Workspace Name"
						class="w-full"
						r#type={InputType::Text}
						id="name"
						name="name"
						value={workspace.get().data.name}
					/>
				</div>
			</div>

			<div class="flex my-xs w-full">
				<div class="flex-2 flex flex-col items-start justify-start mt-md">
					<label
						html_for="alertEmail"
						class="text-white text-sm flex items-center justify-start"
					>
						"Alert Email(s)"
					</label>
					<span class="text-grey">
						"These are a list of emails that will recieve a notification whenever a deployment crashes"
					</span>
				</div>

				<div class="flex-10 flex flex-col items-start justify-start">
					<div class="w-full flex items-center justify-start mb-xs">
						<div class="flex-11">
							<Textbox value={"ac380012@gmail.com".into_view()} />
						</div>
					</div>
				</div>
			</div>
		</div>
	}
}

#[component]
pub fn ManageWorkspaceSettingsTab() -> impl IntoView {
	let (state, _) = AuthState::load();

	let workspace_list = create_resource(
		move || state.get().get_access_token(),
		move |value| async move {
			if let Some(value) = value {
				list_user_workspace(value).await
			} else {
				Err(ServerFnError::WrappedServerError(ErrorType::Unauthorized))
			}
		},
	);

	let current_workspace_id =
		Signal::derive(move || match state.get().get_last_used_workspace_id() {
			Some(id) => Some(id),
			_ => None,
		});

	let current_workspace = Signal::derive(move || {
		if let Some(workspace_id) = current_workspace_id.get() {
			workspace_list
				.get()
				.and_then(|list| {
					list.ok().map(|list| {
						list.workspaces
							.iter()
							.find(|&x| x.id == workspace_id)
							.cloned()
					})
				})
				.flatten()
		} else {
			None
		}
	});

	view! {
		<div class="flex flex-col items-start justify-start w-full h-full fit-wide-screen mx-auto px-md my-xl">
			<Transition>
				{move || match current_workspace.get() {
					Some(current_workspace) => {
						view! { <ShowWorkspaceInfo workspace={current_workspace.clone()} /> }
							.into_view()
					}
					None => view! {}.into_view(),
				}}
			</Transition>
		</div>
	}
}
