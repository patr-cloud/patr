use leptos_router::{
	MatchNestedRoutes,
	components::{Outlet, ParentRoute},
	path,
};
use models::frontend::auth::*;

use crate::prelude::*;

/// The parent component for ALL logged in routes. All logged in content will be
/// nested inside this component
#[component]
pub fn LoggedOutHolder() -> impl IntoView {
	view! {
		<PageContainer class="bg-onboard">
			<Outlet/>
		</PageContainer>
	}
}

/// The content to show when the user is not logged in
#[component(transparent)]
pub fn LoggedOutContent() -> impl MatchNestedRoutes + Clone {
	view! {
		<ParentRoute path=path!("") view=Outlet>
			<AppRoute<LoginRoute, _, _> view=LoginPage />
			<AppRoute<SignUpRoute, _, _> view=SignUpPage />
			// <AppRoute<VerifySignUpRoute, _, _> view=VerifySignUpPage />
			// <AppRoute<ForgotPasswordRoute, _, _> view=ForgotPasswordPage />
		</ParentRoute>
	}
	.into_inner()
}
