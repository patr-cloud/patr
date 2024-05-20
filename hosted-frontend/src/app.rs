use leptos_router::{Outlet, ProtectedRoute, Route, Router, Routes};
use models::api::auth::*;

use crate::{global_state::*, pages::*, prelude::*};

/// Contains all the API endpoints for the application
#[allow(async_fn_in_trait)] // WIP
pub trait AppAPIs {
	/// The login endpoint
	async fn login(
		request: ApiRequest<LoginRequest>,
	) -> Result<AppResponse<LoginRequest>, ServerFnError<ErrorType>>;
}
#[component]
fn LoggedInPage() -> impl IntoView {
	view! {
		<div class="fr-fs-fs full-width full-height bg-secondary">
			<Sidebar />
			<main class="fc-fs-ct full-width px-lg">
				// This is a temporary empty div for the header
				<header style="width: 100%; min-height: 5rem;">
				</header>

				<Outlet />
			</main>
		</div>
	}
}

#[component]
fn InnerApp() -> impl IntoView {
	let (state, _) = get_auth_state();

	let _ = authstate_from_cookie();
	create_effect(move |_| {
		let _ = authstate_from_cookie();
	});

	view! {
		<Router>
			<Routes>
				<ProtectedRoute
					path="/"
					redirect_path="/login"
					condition=move || state.get().is_logged_in()
					view=LoggedInPage
				>
					<ProfileRoutes />
					<InfrastructureRoutes />
					<DomainConfigurationRoutes />
					<Route path="" view=|| view! { <div></div> } />
				</ProtectedRoute>

				<ProtectedRoute
					path="/"
					view=AuthPage
					condition=move || { logging::log!("state: {:#?}", state.get()); !state.get().is_logged_in() }
					redirect_path="/"
				>
					<Route path=LoggedOutRoute::Login view=LoginForm />
					<Route path=LoggedOutRoute::SignUp view=SignUpPage >
						<Route
							path=LoggedOutRoute::ConfirmOtp
							view=ConfirmSignUpForm
						/>
						<Route path=AppRoutes::Empty view=SignUpForm />
					</Route>
				</ProtectedRoute>
			</Routes>
		</Router>
	}
}

#[component]
pub fn App() -> impl IntoView {
	let state = create_rw_signal(GlobalState::new());

	provide_context(state);

	view! {
		<InnerApp />
	}
}
