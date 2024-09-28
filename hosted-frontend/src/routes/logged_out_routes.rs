use leptos_router::{ProtectedRoute, Route};

use crate::{pages::*, prelude::*, routes::LoggedInRoutesView, utils::AuthState};

/// All the routes for when the user is logged out
#[component(transparent)]
pub fn LoggedOutRoutesComponent() -> impl IntoView {
	let (state, _) = AuthState::load();

	view! {
		<ProtectedRoute
			path={AppRoutes::Empty}
			redirect_path={AppRoutes::LoggedInRoute(LoggedInRoute::Home)}
			view={LoggedInRoutesView}
			condition={move || state.get().is_logged_out()}
		>
			<Route path={LoggedOutRoute::Login} view={LoginForm} />
			<Route path={LoggedOutRoute::SignUp} view={SignUpForm} />
			<Route path={LoggedOutRoute::ConfirmOtp} view={ConfirmSignUpPage} />
		</ProtectedRoute>
	}
}
