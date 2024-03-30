use leptos_router::{Outlet, ProtectedRoute, Route, Router, Routes};
use leptos_use::{use_cookie, utils::FromToStringCodec};
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
	let (access_token, _) = use_cookie::<String, FromToStringCodec>("access_token");

	view! {
		<Router>
			<Routes>
				<ProtectedRoute
					path="/"
					redirect_path="/login"
					condition=move || access_token.get().is_some()
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
					condition=move || access_token.get().is_none()
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
