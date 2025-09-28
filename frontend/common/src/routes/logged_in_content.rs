use leptos_router::{
	MatchNestedRoutes,
	components::{Outlet, Route},
	path,
};

use crate::prelude::*;

/// The parent component for ALL logged in routes. All logged in content will be
/// nested inside this component
#[component]
pub fn LoggedInHolder() -> impl IntoView {
	view! {
		<PageContainer class="bg-onboard">
			<Outlet/>
		</PageContainer>
	}
}

/// The content to show when the user is logged in
#[component(transparent)]
pub fn LoggedInContent() -> impl MatchNestedRoutes + Clone {
	view! {
		<Route
			view=Outlet
			path=path!("/")
		/>
	}
	.into_inner()
}
