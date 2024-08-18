use codee::string::FromToStringCodec;
use leptos_query_devtools::LeptosQueryDevtools;
use leptos_router::{Outlet, ProtectedRoute, Route, Router, Routes};

use crate::{pages::*, prelude::*, routes::*, utils::AuthState};

#[component]
pub fn AppOutletView() -> impl IntoView {
	let (state, _) = AuthState::load();

	view! {
		{move || match state.get() {
			AuthState::LoggedOut => view! {
				<PageContainer class="bg-image">
					<Outlet/>
				</PageContainer>
			}.into_view(),
			AuthState::LoggedIn { access_token: _, refresh_token: _, last_used_workspace_id } => {
				if last_used_workspace_id.is_some() {
					view! {
						<div class="fr-fs-fs full-width full-height bg-secondary">
							<Sidebar>
								<div></div>
							</Sidebar>
							<main class="fc-fs-ct full-width px-lg">
								<header style="width: 100%; min-height: 5rem;"></header>

								<Outlet/>
							</main>
						</div>
					}
					.into_view()
				} else {
					view! {
						<div>"No workspace exists. Create workspace"</div>
					}
					.into_view()
				}
			}
		}}
	}
}

#[component]
pub fn AppOutlet() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, set_current_workspace) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let workspace_list = create_resource(
		move || access_token.get(),
		move |value| async move { list_user_workspace(value).await },
	);
	let (state, set_state) = AuthState::load();

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
					set_current_workspace.set(first_id.map(|x| x.to_string()));
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
		<div class="fr-fs-fs full-width full-height bg-secondary">
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
				<Outlet/>
			</main>
		</div>
	}
}

/// The main application component. This is the root component of the
/// application. It contains the main router and all the routes.
#[component]
pub fn App() -> impl IntoView {
	let (state, _) = AuthState::load();

	// TODO: When redirecting to login, the URL should include the path that the
	// user was trying to access. This way, after login, the user is redirected
	// to the page they were trying to access.
	view! {
		<LeptosQueryDevtools />
		<Router>
			<Routes>
				// Logged in routes
				<ProtectedRoute
					path={AppRoutes::Empty}
					view={AppOutlet}
					redirect_path={AppRoutes::LoggedOutRoute(LoggedOutRoute::Login)}
					condition={move || state.get().is_logged_in()}
				>
					<ProfileRoutes/>
					<InfrastructureRoutes/>
					<DomainConfigurationRoutes/>
					<RunnerRoutes />
					<WorkspaceRoutes/>
					<Route path="" view={|| view! { <div></div> }}/>
				</ProtectedRoute>
				<ProtectedRoute
					path={AppRoutes::Empty}
					redirect_path={AppRoutes::LoggedInRoute(LoggedInRoute::Home)}
					view={AppOutletView}
					condition={move || state.get().is_logged_out()}
				>
					<Route path={LoggedOutRoute::Login} view={LoginForm}/>
					<Route path={LoggedOutRoute::SignUp} view={SignUpForm}/>
					<Route path={LoggedOutRoute::ConfirmOtp} view={ConfirmSignUpPage}/>
				</ProtectedRoute>
			</Routes>
		</Router>
	}
}
