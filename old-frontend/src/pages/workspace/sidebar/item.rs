use models::api::workspace::Workspace;

use crate::prelude::*;

#[component]
pub fn WorkspaceItem(
	/// The Workspace Info
	#[prop(into)]
	workspace: MaybeSignal<WithId<Workspace>>,
	/// Set the Current Workspace ID
	#[prop(into)]
	set_workspace_id: WriteSignal<Option<Uuid>>,
	/// Show Switcher Setter
	#[prop(into)]
	show_workspace_switcher: RwSignal<bool>,
) -> impl IntoView {
	// let (state, set_state) = AuthState::load();

	view! {
		<li
			class="full-width py-xxs"
			on:click={
				let workspace = workspace.clone();
				move |_| {
					set_workspace_id.set(Some(workspace.get().id));
					show_workspace_switcher.set(false);
				}
			}
		>
			<div class="flex flex-col justify-between items-start py-xxs px-md br-sm gap-xxs w-full cursor-pointer
			border border-border-color h-full outline-primary-focus workspace-item">
				<div class="flex justify-between items-center w-full gap-xxs">
					<span class="overflow-hidden text-of-ellipsis w-[25ch] text-sm text-medium">
						{workspace.get().name.clone()}
					</span>
				</div>

				<span class="flex justify-start items-center text-xxs text-grey">
					"Owned by &nbsp;" <strong class="text-xxs text-bold text-white">"You"</strong>
				</span>
			</div>
		</li>
	}
}
