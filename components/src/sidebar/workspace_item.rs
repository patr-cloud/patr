use models::{api::workspace::Workspace, prelude::*};

use crate::imports::*;

#[component]
pub fn WorkspaceItem(
	/// The Workspace Info
	#[prop(into)]
	workspace: MaybeSignal<WithId<Workspace>>,
	/// Set the Current Workspace ID
	#[prop(into)]
	set_workspace_id: WriteSignal<Option<String>>,
) -> impl IntoView {
	view! {
		<li class="full-width py-xxs"
			on:click={
				let workspace = workspace.clone();
				move |_| {
					logging::log!("{:#?}", workspace.get());
					set_workspace_id.set(Some(workspace.get().id.to_string()))
				}
			}
		>
			<div
				class="fc-sb-fs py-xxs px-md br-sm gap-xxs full-width workspace-item cursor-pointer bd-light full-height outline-primary-focus"
			>
				<div class="fr-sb-ct full-width gap-xxs">
					<span class="of-hidden txt-of-ellipsis w-25 txt-sm txt-medium">
						{workspace.get().name.clone()}
					</span>
				</div>

				<span class="fr-fs-ct txt-xxs txt-grey">
					"Owned by &nbsp;"
					<strong class="txt-xxs txt-bold txt-white">"You"</strong>
				</span>
			</div>
		</li>
	}
}
