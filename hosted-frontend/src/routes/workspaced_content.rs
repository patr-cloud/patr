use codee::string::FromToStringCodec;
use leptos_router::{Outlet, ProtectedRoute, Route, Router, Routes};

use crate::{
	app::{AppOutlet, AppOutletView},
	pages::*,
	prelude::*,
	utils::AuthState,
};

#[component]
pub fn WorkspacedRouteView() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, set_current_workspace) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let workspace_list = create_resource(
		move || access_token.get(),
		move |value| async move { list_user_workspace(value).await },
	);

	let current_workspace_id = Signal::derive(move || {
		match current_workspace_id.with(|id| id.clone().map(|id| Uuid::parse_str(id.as_str()))) {
			Some(Ok(id)) => Some(id),
			_ => {
				let first_id = workspace_list.get().and_then(|list| {
					list.ok().and_then(|x| {
						let x = x.workspaces.first().and_then(|x| Some(x.id));
						x
					})
				});
				set_current_workspace.set(first_id.map(|x| x.to_string()));

				first_id
			}
		}
	});

	let current_workspace = Signal::derive(move || {
		if let Some(workspace_id) = current_workspace_id.get() {
			workspace_list
				.get()
				.map(|list| {
					list.ok().map(|list| {
						list.workspaces
							.iter()
							.find(|&x| x.id == workspace_id)
							.cloned()
					})
				})
				.flatten()
				.flatten()
		} else {
			None
		}
	});

	view! {
		<Sidebar>
			<Transition>
				{
					move || match workspace_list.get() {
						Some(workspace_list) => {
							match workspace_list {
								Ok(data) => {
									view! {
										<WorkspaceCard
											current_workspace={current_workspace}
											set_workspace_id={set_current_workspace}
											workspaces={data.clone().workspaces}
										/>
									}.into_view()
								},
								Err(_) => view! {"Error Loading"}.into_view()
							}
						},
						None => view! {"loading..."}.into_view()
					}
				}
			</Transition>
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

	let workspace_list = create_resource(
		move || state.get().get_access_token(),
		move |value| async move { list_user_workspace(value).await },
	);

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
						last_used_workspace_id,
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
			view={move || view! {
				<WorkspacedRouteView />
			}}
			redirect_path={AppRoutes::LoggedInRoute(LoggedInRoute::UserProfile)}
			condition={move || current_workspace_id.get().is_some()}
		>
			<InfrastructureRoutes/>
			<DomainConfigurationRoutes/>
			<RunnerRoutes />
		</ProtectedRoute>
	}
}