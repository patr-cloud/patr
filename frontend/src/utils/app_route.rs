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
	/// The scope of the component
	cx: Scope,
	/// The path of the route
	_route: R,
	/// The view for the route
	view: F,
	/// The children of the route
	children: Children,
) -> impl IntoView
where
	R: TypedRoute,
	F: Fn(Scope) -> V + 'static,
	V: IntoView,
{
	view! { cx,
		<Route
			view={move |cx| {
				let query: R::Query = use_router_query(cx)
					.get_untracked()
					.unwrap_or_default();
				let params: R = use_router_params(cx)
					.get_untracked()
					.unwrap_or_default();
				provide_context(cx, Query(query));
				provide_context(cx, UrlParams(params));
				view(cx)
			}}
			path={<R as TypedPath>::PATH}>
			{children(cx)}
		</Route>
	}
}

/// Get the query parameters for the current route.
pub fn use_query<R>(cx: Scope) -> R::Query
where
	R: TypedRoute,
{
	expect_context::<Query<R::Query>>(cx).0
}

/// Get the path parameters for the current route.
pub fn use_params<R>(cx: Scope) -> R
where
    R: TypedRoute,
{
    expect_context::<UrlParams<R>>(cx).0
}
