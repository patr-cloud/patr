use leptos_router::*;

use crate::{global_state::*, prelude::*};

mod confirm_sign_up;
mod login;
mod sign_up;

pub use self::{confirm_sign_up::*, login::*, sign_up::*};

#[component(transparent)]
pub fn AuthRoutes() -> impl IntoView {
	let (auth, _) = get_auth_state();

	view! {
		<ProtectedRoute
			path="/"
			view={AuthPage}
			condition={move || !auth.get().is_logged_in()}
			redirect_path="/"
		>
			<Route path={LoggedOutRoute::Login} view={LoginForm}/>
			<Route path={LoggedOutRoute::SignUp} view={SignUpPage}>
				<Route path={LoggedOutRoute::ConfirmOtp} view={ConfirmSignUpForm}/>
				<Route path={AppRoutes::Empty} view={SignUpForm}/>
			</Route>
		</ProtectedRoute>
	}
}
