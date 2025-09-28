use leptos_router::{
	MatchNestedRoutes,
	components::{Outlet, ParentRoute, Route},
	path,
};
use models::frontend::auth::*;

use crate::prelude::*;

/// The content to show when the user is not logged in
#[component(transparent)]
pub fn LoggedOutContent() -> impl MatchNestedRoutes + Clone {
	view! {
		<ParentRoute path=path!("") view=Outlet>
			<AppRoute<LoginRoute, _, _> view=LoginPage />
			<Route
				view=SignUpPage
				path=path!("/sign-up")
			/>
			// <Route
			// 	view=ForgotPasswordPage
			// 	path=path!("/forgot-password")
			// />
			// <Route
			// 	view=ConfirmSignUpPage
			// 	path=path!("/confirm-sign-up")
			// />
		</ParentRoute>
	}
	.into_inner()
}
