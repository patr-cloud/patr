mod card;
mod item;
mod switcher;

use card::WorkspaceCard;

use crate::{prelude::*, queries::list_workspaces_query, utils::AuthState};

/// The Component that renders the workspace viewer and switcher in sidebar
#[component]
pub fn WorkspaceSidebarComponent() -> impl IntoView {
	let workspace_list = list_workspaces_query();
	let (state, set_state) = AuthState::load();
	let (current_workspace, set_current_workspace) = create_signal(None);

	create_effect(move |_| {
		if let Some(current_workspace) = current_workspace.get() {
			set_state.update(|state| {
				if let Some(AuthState::LoggedIn {
					ref mut last_used_workspace_id,
					..
				}) = *state
				{
					*last_used_workspace_id = Some(current_workspace);
				}
			})
		}
	});

	let current_workspace_id =
		Signal::derive(
			move || match state.with(|state| state.get_last_used_workspace_id()) {
				Some(id) => Some(id),
				_ => {
					let first_id = workspace_list.get().and_then(|list| {
						list.ok().and_then(|x| {
							let x = x.workspaces.first().and_then(|x| Some(x.id));
							x
						})
					});
					set_current_workspace.set(first_id);
					set_state.update(|state| match *state {
						Some(AuthState::LoggedIn {
							ref mut last_used_workspace_id,
							..
						}) => {
							*last_used_workspace_id = first_id;
						}
						_ => {}
					});

					first_id
				}
			},
		);

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
		<Transition>
			{move || match workspace_list.get() {
				Some(workspace_list) => {
					match workspace_list {
						Ok(data) => {
							view! {
								<WorkspaceCard
									current_workspace={current_workspace}
									set_workspace_id={set_current_workspace}
									workspaces={data.clone().workspaces}
								/>
							}
								.into_view()
						}
						Err(err) => format!("Error Loading, {:?}", err).into_view(),
					}
				}
				None => view! { "loading..." }.into_view(),
			}}
		</Transition>
	}
}
