use leptos_router::{Outlet, ProtectedRoute, Route, Router, Routes};
use models::api::auth::*;

use crate::{global_state::*, pages::*, prelude::*};

#[allow(async_fn_in_trait)] // WIP
pub trait AppAPIs {
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
pub fn App() -> impl IntoView {
	let state = create_rw_signal(AuthState::LoggedOut);

	provide_context(state);

	let (auth, _) = get_auth_state();

	view! {
		<Router>
			<Routes>
				<AuthRoutes />
				<ProtectedRoute
					path="/"
					redirect_path="/login"
					condition=move || auth.get().is_logged_in()
					view=LoggedInPage
				>
					<ProfileRoutes />
					<InfrastructureRoutes />
					<DomainConfigurationRoutes />
					<Route path="" view=|| view! { <div></div> } />
				</ProtectedRoute>
			</Routes>
		</Router>
	}
}
