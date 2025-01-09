use leptos_router::{Outlet, ProtectedRoute};

use crate::{pages::*, prelude::*, queries::list_workspaces_query, utils::AuthState};

/// The view for the Workspaced Routes
#[component]
pub fn WorkspacedRouteView() -> impl IntoView {
	view! {
		<Sidebar sidebar_items={get_sidebar_items(expect_context::<AppType>())}>
			<WorkspaceSidebarComponent />
		</Sidebar>

		<main class="fc-fs-ct full-width px-lg">
			<Outlet />
		</main>
	}
}

/// Contains all the Workspaced Routes, i.e. routes that require a workspace
#[component(transparent)]
pub fn WorkspacedRoutes() -> impl IntoView {
	let (state, set_state) = AuthState::load();

	let workspace_list = list_workspaces_query();

	let current_workspace_id =
		Signal::derive(move || match state.get().get_last_used_workspace_id() {
			Some(id) => Some(id),
			_ => {
				let first_id = workspace_list.get().and_then(|list| {
					list.ok().and_then(|x| {
						let x = x.workspaces.first().and_then(|x| Some(x.id));
						x
					})
				});
				let new_state = match state.get() {
					AuthState::LoggedOut => AuthState::LoggedOut,
					AuthState::LoggedIn {
						last_used_workspace_id: _,
						access_token,
						refresh_token,
					} => AuthState::LoggedIn {
						access_token,
						refresh_token,
						last_used_workspace_id: first_id.clone(),
					},
				};
				logging::log!("{:?}", new_state);
				set_state.set(Some(new_state));
				// set_current_workspace.set(first_id.map(|x| x.to_string()));

				first_id
			}
		});

	view! {
		<ProtectedRoute
			path={AppRoutes::Empty}
			view={WorkspacedRouteView}
			redirect_path={AppRoutes::LoggedInRoute(LoggedInRoute::UserProfile)}
			condition={move || current_workspace_id.get().is_some()}
		>
			<InfrastructureRoutes />
			<DomainConfigurationRoutes />
			<RunnerRoutes />
		</ProtectedRoute>
	}
}
