use leptos_router::{Outlet, ProtectedRoute, Route, Router, Routes};

use crate::{pages::*, prelude::*, utils::AuthState};

#[component]
fn AppOutletView() -> impl IntoView {
	let AuthStateContext(state) = expect_context::<AuthStateContext>();

	view! {
		{move || match state.get() {
			AuthState::LoggedOut => view! {
				<PageContainer class="bg-image">
					<Outlet/>
				</PageContainer>
			}.into_view(),
			AuthState::LoggedIn { access_token: _, refresh_token: _, last_used_workspace_id } => {
				if let Some(_) = last_used_workspace_id {
					view! {
						<div class="fr-fs-fs full-width full-height bg-secondary">
							<Sidebar/>
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

/// The main application component. This is the root component of the
/// application. It contains the main router and all the routes.
#[component]
pub fn App() -> impl IntoView {
	let state = create_rw_signal(AuthState::load());

	provide_context::<AuthStateContext>(AuthStateContext(state));

	// TODO: When redirecting to login, the URL should include the path that the
	// user was trying to access. This way, after login, the user is redirected
	// to the page they were trying to access.
	view! {
		<Router>
			<Routes>
				// Logged in routes
				<ProtectedRoute
					path={AppRoutes::Empty}
					view={AppOutletView}
					redirect_path={AppRoutes::LoggedOutRoute(LoggedOutRoute::Login)}
					condition={move || state.get().is_logged_in()}
				>
					<ProfileRoutes/>
					<InfrastructureRoutes/>
					<DomainConfigurationRoutes/>
					<WorkspaceRoutes/>
					<Route path="" view={|| view! { <div></div> }}/>
				</ProtectedRoute>

				// Logged out routes
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
