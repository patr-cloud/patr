use leptos_router::{
	MatchNestedRoutes,
	components::{Outlet, Route},
	path,
};

use crate::prelude::*;

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
