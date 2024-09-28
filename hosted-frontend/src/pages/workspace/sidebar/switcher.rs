use models::api::workspace::Workspace;

use super::item::WorkspaceItem;
use crate::prelude::*;

#[component]
pub fn WorkspaceSwitcher(
	/// List of workspaces
	#[prop(into)]
	workspaces: MaybeSignal<Vec<WithId<Workspace>>>,
	/// Set the Current Workspace ID
	#[prop(into)]
	set_workspace_id: WriteSignal<Option<Uuid>>,
	/// Show Switcher Setter
	#[prop(into)]
	show_workspace_switcher: RwSignal<bool>,
) -> impl IntoView {
	let stored_workspaces = store_value(workspaces);

	view! {
		<Portal>
			<div
				tab_index={-1}
				class="
				text-white bg-secondary-light border border-border-color rounded-sm
				flex flex-col itmes-start justify-start workspace-switcher pt-md "
			>
				<p class="mx-xl text-md mb-sm">"Workspaces"</p>
				<div class="fc-fs-fs w-full ul-light pb-xs">
					<ul class="w-full overflow-y-auto px-xl flex flex-col items-start justify-start">
						<For
							each={move || {
								stored_workspaces.with_value(|workspaces| workspaces.clone().get())
							}}
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

				<div class="flex flex-col items-center justify-center w-full my-lg">
					<Link
						style_variant={LinkStyleVariant::Plain}
						r#type={Variant::Link}
						to="/workspace/create"
						class="gap-xxs"
					>
						"CREATE WORKSPACE"
						<Icon
							icon={IconType::Plus}
							size={Size::ExtraSmall}
							color={Color::Primary}
						/>
					</Link>
				</div>
			</div>
		</Portal>
	}
}
