use leptos_router::use_navigate;
use models::api::{workspace::Workspace, WithId};

use crate::imports::*;

#[component]
pub fn WorkspaceCard(
	/// Additional classes to apply
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The Workspace
	#[prop(into)]
	workspaces: MaybeSignal<Vec<WithId<Workspace>>>,
	/// The Currently Selected Workspace
	#[prop(into, optional)]
	current_workspace: MaybeSignal<Option<WithId<Workspace>>>,
	/// Set the Current Workspace ID
	#[prop(into)]
	set_workspace_id: WriteSignal<Option<String>>,
) -> impl IntoView {
	let show_workspace_switcher = create_rw_signal(false);
	let navigate = use_navigate();

	view! {
		<div
			class="sidebar-user fr-sb-ct py-sm px-md cursor-pointer full-width br-sm bg-secondary-dark gap-xxs pos-rel "
			on:click={move |ev| {
				show_workspace_switcher.update(|v| *v = !*v)
			}}
		>
			<div class="fc-fs-fs full-width">
				<p class="txt-sm txt-white w-20 txt-of-ellipsis of-hidden">
					{move || match current_workspace.get() {
						Some(workspace) => {
							format!("{}", workspace.name).into_view()
						},
						None => "Select A Workspace".into_view()
					}}
				</p>
			</div>

			<Link
				r#type={Variant::Button}
				to="/workspace"
			>
				<Icon icon=IconType::Settings color=Color::Grey />
			</Link>

			<Show when=move || show_workspace_switcher.get()>
				<WorkspaceSwitcher
					set_workspace_id={set_workspace_id.clone()}
					workspaces={workspaces.clone()}
					show_workspace_switcher={show_workspace_switcher}
				/>
			</Show>
		</div>
	}
}
