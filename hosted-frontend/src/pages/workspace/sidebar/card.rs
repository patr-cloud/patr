use leptos_router::use_navigate;
use models::api::workspace::Workspace;

use super::switcher::WorkspaceSwitcher;
use crate::prelude::*;

/// The Workspace Card on the sidebar
#[component]
pub fn WorkspaceCard(
	/// The Workspace
	#[prop(into)]
	workspaces: MaybeSignal<Vec<WithId<Workspace>>>,
	/// The Currently Selected Workspace
	#[prop(into, optional)]
	current_workspace: MaybeSignal<Option<WithId<Workspace>>>,
	/// Set the Current Workspace ID
	#[prop(into)]
	set_workspace_id: WriteSignal<Option<Uuid>>,
) -> impl IntoView {
	let show_workspace_switcher = create_rw_signal(false);
	let _navigate = use_navigate();

	view! {
		<div
			class="sidebar-user flex justify-between items-center py-sm px-md cursor-pointer
			w-full br-sm bg-secondary-dark gap-xxs relative"
			on:click={move |_| { show_workspace_switcher.update(|v| *v = !*v) }}
		>
			<div class="flex flex-col items-start justify-start w-full">
				<p class="text-sm text-white w-[20ch] text-ellipsis overflow-hidden">
					{move || match current_workspace.get() {
						Some(workspace) => format!("{}", workspace.name).into_view(),
						None => "Select A Workspace".into_view(),
					}}
				</p>
			</div>

			<Link r#type={Variant::Button} to="/workspace">
				<Icon icon={IconType::Settings} color={Color::Grey} />
			</Link>

			<Show when={move || show_workspace_switcher.get()}>
				<WorkspaceSwitcher
					set_workspace_id={set_workspace_id.clone()}
					workspaces={workspaces.clone()}
					show_workspace_switcher={show_workspace_switcher}
				/>
			</Show>
		</div>
	}
}
