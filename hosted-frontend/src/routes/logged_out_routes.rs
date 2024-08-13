use leptos_router::{Outlet, ProtectedRoute, Route, Router, Routes};

use crate::{
	app::{AppOutlet, AppOutletView},
	pages::*,
	prelude::*,
	routes::LoggedInRoutesView,
	utils::AuthState,
};

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
			<Route path={LoggedOutRoute::Login} view={LoginForm}/>
			<Route path={LoggedOutRoute::SignUp} view={SignUpForm}/>
			<Route path={LoggedOutRoute::ConfirmOtp} view={ConfirmSignUpPage}/>
		</ProtectedRoute>
	}
}
