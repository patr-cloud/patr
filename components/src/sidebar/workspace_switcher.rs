use models::{api::workspace::Workspace, prelude::*};

use super::WorkspaceItem;
use crate::imports::*;

#[component]
pub fn WorkspaceSwitcher(
	/// List of workspaces
	#[prop(into)]
	workspaces: MaybeSignal<Vec<WithId<Workspace>>>,
	/// Set the Current Workspace ID
	#[prop(into)]
	set_workspace_id: WriteSignal<Option<String>>,
	/// Show Switcher Setter
	#[prop(into)]
	show_workspace_switcher: RwSignal<bool>,
) -> impl IntoView {
	let stored_workspaces = store_value(workspaces);

	view! {
		<Portal>
			<div
				tab_index={-1}
				class="txt-white bg-secondary-light bd-light br-sm pt-md fc-fs-fs workspace-switcher"
			>
				<p class="mx-xl txt-md mb-sm">"Workspaces"</p>
				<div class="fc-fs-fs full-width ul-light pb-xs">
					<ul class="full-width ofy-auto px-xl fc-fs-fs">
						<For
							each={move || stored_workspaces.with_value(|workspaces| workspaces.clone().get())}
							key={|state| state.id}
							let:child
						>
							<WorkspaceItem
								show_workspace_switcher={show_workspace_switcher}
								set_workspace_id={set_workspace_id.clone()}
								workspace={child}
							/>
						</For>
					</ul>
				</div>

				<div class="fc-ct-ct full-width my-lg">
					<Link
						style_variant={LinkStyleVariant::Plain}
						r#type={Variant::Link}
						to="/workspace/create"
						class="gap-xxs"
					>
						"CREATE WORKSPACE" <Icon icon=IconType::Plus size=Size::ExtraSmall color=Color::Primary />
					</Link>
				</div>
			</div>
		</Portal>
	}
}
