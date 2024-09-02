use models::api::workspace::Workspace;

use crate::prelude::*;

#[component]
pub fn WorkspaceItem(
	/// The Workspace Info
	#[prop(into)]
	workspace: MaybeSignal<WithId<Workspace>>,
	/// Set the Current Workspace ID
	#[prop(into)]
	set_workspace_id: WriteSignal<Option<String>>,
	/// Show Switcher Setter
	#[prop(into)]
	show_workspace_switcher: RwSignal<bool>,
) -> impl IntoView {
	// let (state, set_state) = AuthState::load();

	view! {
		<li class="full-width py-xxs"
			on:click={
				let workspace = workspace.clone();
				move |_| {
					logging::log!("{:#?}", workspace.get());
					set_workspace_id.set(Some(workspace.get().id.to_string()));
					show_workspace_switcher.set(false);
					// set_state.update(|state| match *state {
					// 	Some(AuthState::LoggedIn {
					// 		ref mut last_used_workspace_id,
					// 		..
					// 	}) => {
					// 		*last_used_workspace_id = Some(workspace.get().id.to_string());
					// 	}
					// 	_ => {}
					// });
				}
			}
		>
			<div
				class="flex flex-col justify-between items-start py-xxs px-md br-sm gap-xxs w-full cursor-pointer
				border border-border-color h-full outline-primary-focus workspace-item"
			>
				<div class="flex justify-between items-center w-full gap-xxs">
					<span class="overflow-hidden text-of-ellipsis w-[25ch] text-sm text-medium">
						{workspace.get().name.clone()}
					</span>
				</div>

				<span class="flex justify-start items-center text-xxs text-grey">
					"Owned by &nbsp;"
					<strong class="text-xxs text-bold text-white">"You"</strong>
				</span>
			</div>
		</li>
	}
}
