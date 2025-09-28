use std::marker::PhantomData;

use leptos_router::{
	MatchNestedRoutes,
	components::Route,
	hooks::{use_params as use_router_params, use_query as use_router_query},
};
use models::frontend::TypedRoute;

use crate::prelude::*;

/// A component that renders a route based on the given typed route. It extracts
/// the query parameters and the route parameters from the router hooks and
/// passes them to the view function.
#[component(transparent)]
pub fn AppRoute<R, F, V>(
	/// Phantom data for the route
	#[prop(optional)]
	_phantom: PhantomData<R>,
	/// The view for the route
	view: F,
) -> impl MatchNestedRoutes + Clone
where
	R: TypedRoute,
	F: Fn(R::Query, R::Path) -> V + Clone + Send + 'static,
	V: IntoView,
{
	let query: R::Query = use_router_query().get_untracked().unwrap_or_default();
	let params: R::Path = use_router_params()
		.get_untracked()
		.expect("cannot parse params");

	view! {
		<Route view={move || view(query.clone(), params.clone())} path={R::leptos_path()} />
	}
	.into_inner()
}
