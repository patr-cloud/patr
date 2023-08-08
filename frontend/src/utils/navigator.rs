use leptos_router::{NavigateOptions, NavigationError};

use crate::prelude::*;

/// A hook to navigate to a route. Must provide an AppRoute to navigate to.
pub fn use_navigate(
	cx: Scope,
) -> impl Fn(AppRoute) -> Result<(), NavigationError> {
	let navigate = leptos_router::use_navigate(cx);

	move |route| {
		navigate(route.to_string().as_str(), NavigateOptions::default())
	}
}
