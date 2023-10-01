use axum_extra::routing::TypedPath;
use leptos_router::{
	use_params as use_router_params,
	use_query as use_router_query,
	Params,
	Route,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::prelude::*;

/// A trait for types that can be used as a route in the application.
/// It also provides the path as well as the query parameters for the route.
pub trait TypedRoute:
	TypedPath
	+ Params
	+ DeserializeOwned
	+ Serialize
	+ PartialEq
	+ Default
	+ Clone
	+ 'static
{
	/// The query parameters for the route.
	type Query: Params
		+ DeserializeOwned
		+ Serialize
		+ PartialEq
		+ Clone
		+ Default
		+ 'static;
}

/// A wrapper around a type that implements `TypedRoute` to provide the query
/// parameters for the route through the context.
#[derive(Debug, Clone)]
struct Query<T>(T);
/// A wrapper around a type that implements `TypedRoute` to provide the path
/// parameters for the route through the context.
#[derive(Debug, Clone)]
struct UrlParams<T>(T);

#[component]
pub fn AppRoute<R, F, V>(
	/// The path of the route
	_route: R,
	/// The view for the route
	view: F,
	/// The children of the route
	children: Children,
) -> impl IntoView
where
	R: TypedRoute,
	F: Fn() -> V + 'static,
	V: IntoView,
{
	view! {
		<Route
			view={move || {
				let query: R::Query = use_router_query()
					.get_untracked()
					.unwrap_or_default();
				let params: R = use_router_params()
					.get_untracked()
					.unwrap_or_default();
				provide_context(Query(query));
				provide_context(UrlParams(params));
				view()
			}}
			path={<R as TypedPath>::PATH}>
			{children()}
		</Route>
	}
}

/// Get the query parameters for the current route.
pub fn use_query<R>() -> R::Query
where
	R: TypedRoute,
{
	expect_context::<Query<R::Query>>().0
}

/// Get the path parameters for the current route.
pub fn use_params<R>() -> R
where
    R: TypedRoute,
{
    expect_context::<UrlParams<R>>().0
}
